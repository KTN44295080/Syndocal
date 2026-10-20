use super::{error, IsfControlDefinition, IsfControlKind, IsfPrepareError};
use image::ImageFormat;
use serde_json::{Map, Value};
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Component, Path};
use std::sync::Arc;

const MAX_EXPRESSION_BYTES: usize = 1_024;
const MAX_EXPRESSION_NODES: usize = 256;
const MAX_EXPRESSION_DEPTH: usize = 32;
const MAX_IMPORTED_IMAGES: usize = 15;
const MAX_IMPORTED_FILE_BYTES: u64 = 16 * 1_024 * 1_024;
const MAX_IMPORTED_RGBA_BYTES: u64 = 64 * 1_024 * 1_024;
const MAX_IMPORTED_DIMENSION: u32 = 8_192;

/// Bounded arithmetic evaluated once per frame against frame size and scalar
/// controls. Fractional dimensions are truncated to whole pixels.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct IsfDimension {
    expression: DimensionExpression,
}

impl IsfDimension {
    pub(crate) fn parse(value: Option<&Value>) -> Result<Option<Self>, IsfPrepareError> {
        let Some(value) = value else {
            return Ok(None);
        };
        let expression = match value {
            Value::Number(number) => {
                let number = number
                    .as_f64()
                    .ok_or_else(|| error("ISF pass size number is outside the supported range"))?;
                dimension_pixels(number)?;
                DimensionExpression::Number(number)
            }
            Value::String(source) => {
                if source.is_empty() || source.len() > MAX_EXPRESSION_BYTES || !source.is_ascii() {
                    return Err(error(format!(
                        "ISF pass size expression must contain 1 to {MAX_EXPRESSION_BYTES} ASCII bytes"
                    )));
                }
                let mut parser = DimensionParser {
                    source: source.as_bytes(),
                    position: 0,
                    nodes: 0,
                };
                let expression = parser.sum(0)?;
                parser.skip_space();
                if parser.position != parser.source.len() {
                    return Err(parser.failure("unexpected trailing text"));
                }
                expression
            }
            _ => {
                return Err(error(
                    "ISF pass size must be a number or an arithmetic expression string",
                ))
            }
        };
        Ok(Some(Self { expression }))
    }

    pub(crate) fn evaluate(
        &self,
        width: u32,
        height: u32,
        definitions: &[IsfControlDefinition],
        values: &[[f32; 4]],
    ) -> Result<u32, IsfPrepareError> {
        dimension_pixels(
            self.expression
                .evaluate(width, height, definitions, values)?,
        )
    }
}

fn dimension_pixels(value: f64) -> Result<u32, IsfPrepareError> {
    if !value.is_finite() || value < 1.0 || value > f64::from(u32::MAX) {
        return Err(error(
            "ISF pass size must resolve to a finite value between 1 and 4294967295 pixels; check WIDTH/HEIGHT and the scalar controls",
        ));
    }
    Ok(value.trunc() as u32)
}

fn finite_expression_value(value: f64) -> Result<f64, IsfPrepareError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(error("ISF pass size expression produced a non-finite value; check arithmetic overflow and function arguments"))
    }
}

#[derive(Debug, Clone, PartialEq)]
enum DimensionExpression {
    Number(f64),
    Variable(String),
    Negate(Box<Self>),
    Binary(BinaryOperation, Box<Self>, Box<Self>),
    Function(DimensionFunction, Vec<Self>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Power,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DimensionFunction {
    Min,
    Max,
    Floor,
    Ceil,
    Abs,
    Round,
    Trunc,
    Pow,
    Sqrt,
    Clamp,
}

impl DimensionFunction {
    fn parse(name: &str) -> Result<Self, IsfPrepareError> {
        match name {
            "min" => Ok(Self::Min), "max" => Ok(Self::Max), "floor" => Ok(Self::Floor),
            "ceil" => Ok(Self::Ceil), "abs" => Ok(Self::Abs), "round" => Ok(Self::Round),
            "trunc" => Ok(Self::Trunc), "pow" => Ok(Self::Pow), "sqrt" => Ok(Self::Sqrt),
            "clamp" => Ok(Self::Clamp),
            _ => Err(error(format!(
                "ISF pass size expression function '{name}' is unsupported; use min, max, floor, ceil, abs, round, trunc, pow, sqrt or clamp"
            ))),
        }
    }

    fn argument_count(self) -> usize {
        match self {
            Self::Min | Self::Max | Self::Pow => 2,
            Self::Clamp => 3,
            Self::Floor | Self::Ceil | Self::Abs | Self::Round | Self::Trunc | Self::Sqrt => 1,
        }
    }
}

impl DimensionExpression {
    fn evaluate(
        &self,
        width: u32,
        height: u32,
        definitions: &[IsfControlDefinition],
        values: &[[f32; 4]],
    ) -> Result<f64, IsfPrepareError> {
        let evaluate = |expression: &Self| expression.evaluate(width, height, definitions, values);
        let value = match self {
            Self::Number(value) => *value,
            Self::Variable(name) if name == "WIDTH" => f64::from(width),
            Self::Variable(name) if name == "HEIGHT" => f64::from(height),
            Self::Variable(name) => {
                let (index, definition) = definitions.iter().enumerate()
                    .find(|(_, definition)| definition.name == *name)
                    .ok_or_else(|| error(format!(
                        "ISF pass size expression variable '${name}' is unknown; use $WIDTH, $HEIGHT or a declared scalar input"
                    )))?;
                if matches!(
                    definition.kind,
                    IsfControlKind::Point2d | IsfControlKind::Color
                ) {
                    return Err(error(format!(
                        "ISF pass size expression variable '${name}' is not scalar; point2D and color inputs cannot define a pass dimension"
                    )));
                }
                f64::from(values.get(index).unwrap_or(&definition.default)[0])
            }
            Self::Negate(expression) => -evaluate(expression)?,
            Self::Binary(operation, left, right) => {
                let left = evaluate(left)?;
                let right = evaluate(right)?;
                match operation {
                    BinaryOperation::Add => left + right,
                    BinaryOperation::Subtract => left - right,
                    BinaryOperation::Multiply => left * right,
                    BinaryOperation::Divide | BinaryOperation::Remainder if right == 0.0 => {
                        return Err(error(
                            "ISF pass size expression divides by zero; check its scalar controls",
                        ));
                    }
                    BinaryOperation::Divide => left / right,
                    BinaryOperation::Remainder => left % right,
                    BinaryOperation::Power => left.powf(right),
                }
            }
            Self::Function(function, arguments) => {
                // Parse-time arity validation avoids temporary render-path allocations.
                let first = evaluate(&arguments[0])?;
                match function {
                    DimensionFunction::Floor => first.floor(),
                    DimensionFunction::Ceil => first.ceil(),
                    DimensionFunction::Abs => first.abs(),
                    DimensionFunction::Round => first.round(),
                    DimensionFunction::Trunc => first.trunc(),
                    DimensionFunction::Sqrt => first.sqrt(),
                    DimensionFunction::Min => first.min(evaluate(&arguments[1])?),
                    DimensionFunction::Max => first.max(evaluate(&arguments[1])?),
                    DimensionFunction::Pow => first.powf(evaluate(&arguments[1])?),
                    DimensionFunction::Clamp => {
                        let low = evaluate(&arguments[1])?;
                        let high = evaluate(&arguments[2])?;
                        if low > high {
                            return Err(error(
                                "ISF pass size expression clamp minimum exceeds its maximum",
                            ));
                        }
                        first.clamp(low, high)
                    }
                }
            }
        };
        finite_expression_value(value)
    }
}

struct DimensionParser<'a> {
    source: &'a [u8],
    position: usize,
    nodes: usize,
}

impl DimensionParser<'_> {
    fn failure(&self, detail: &str) -> IsfPrepareError {
        error(format!(
            "ISF pass size expression is invalid at byte {}: {detail}",
            self.position + 1
        ))
    }
    fn check_depth(&self, depth: usize) -> Result<(), IsfPrepareError> {
        if depth > MAX_EXPRESSION_DEPTH {
            Err(self.failure("nesting exceeds the 32-level safety limit"))
        } else {
            Ok(())
        }
    }
    fn node(
        &mut self,
        expression: DimensionExpression,
    ) -> Result<DimensionExpression, IsfPrepareError> {
        self.nodes += 1;
        if self.nodes > MAX_EXPRESSION_NODES {
            Err(self.failure("expression exceeds the 256-node safety limit"))
        } else {
            Ok(expression)
        }
    }
    fn skip_space(&mut self) {
        while self
            .source
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }
    fn consume(&mut self, character: u8) -> bool {
        self.skip_space();
        if self.source.get(self.position) == Some(&character) {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn sum(&mut self, depth: usize) -> Result<DimensionExpression, IsfPrepareError> {
        self.check_depth(depth)?;
        let mut expression = self.product(depth)?;
        loop {
            let operation = if self.consume(b'+') {
                BinaryOperation::Add
            } else if self.consume(b'-') {
                BinaryOperation::Subtract
            } else {
                break;
            };
            let right = self.product(depth)?;
            expression = self.node(DimensionExpression::Binary(
                operation,
                Box::new(expression),
                Box::new(right),
            ))?;
        }
        Ok(expression)
    }
    fn product(&mut self, depth: usize) -> Result<DimensionExpression, IsfPrepareError> {
        let mut expression = self.unary(depth)?;
        loop {
            let operation = if self.consume(b'*') {
                BinaryOperation::Multiply
            } else if self.consume(b'/') {
                BinaryOperation::Divide
            } else if self.consume(b'%') {
                BinaryOperation::Remainder
            } else {
                break;
            };
            let right = self.unary(depth)?;
            expression = self.node(DimensionExpression::Binary(
                operation,
                Box::new(expression),
                Box::new(right),
            ))?;
        }
        Ok(expression)
    }
    fn unary(&mut self, depth: usize) -> Result<DimensionExpression, IsfPrepareError> {
        self.check_depth(depth)?;
        if self.consume(b'+') {
            return self.unary(depth + 1);
        }
        if self.consume(b'-') {
            let expression = self.unary(depth + 1)?;
            return self.node(DimensionExpression::Negate(Box::new(expression)));
        }
        self.power(depth)
    }
    fn power(&mut self, depth: usize) -> Result<DimensionExpression, IsfPrepareError> {
        let expression = self.primary(depth)?;
        if self.consume(b'^') {
            let right = self.unary(depth + 1)?;
            self.node(DimensionExpression::Binary(
                BinaryOperation::Power,
                Box::new(expression),
                Box::new(right),
            ))
        } else {
            Ok(expression)
        }
    }
    fn primary(&mut self, depth: usize) -> Result<DimensionExpression, IsfPrepareError> {
        self.check_depth(depth)?;
        if self.consume(b'(') {
            let expression = self.sum(depth + 1)?;
            if !self.consume(b')') {
                return Err(self.failure("expected ')'"));
            }
            return Ok(expression);
        }
        self.skip_space();
        if self
            .source
            .get(self.position)
            .is_some_and(|value| value.is_ascii_digit() || *value == b'.')
        {
            return self.number();
        }
        let variable_prefix = self.consume(b'$');
        let name = self.identifier()?;
        if !variable_prefix && self.consume(b'(') {
            let function = DimensionFunction::parse(&name)?;
            let mut arguments = Vec::with_capacity(function.argument_count());
            if !self.consume(b')') {
                loop {
                    if arguments.len() >= function.argument_count() {
                        return Err(self.failure("function has too many arguments"));
                    }
                    arguments.push(self.sum(depth + 1)?);
                    if self.consume(b')') {
                        break;
                    }
                    if !self.consume(b',') {
                        return Err(self.failure("expected ',' or ')'"));
                    }
                }
            }
            if arguments.len() != function.argument_count() {
                return Err(error(format!(
                    "ISF pass size expression function '{name}' requires {} arguments",
                    function.argument_count()
                )));
            }
            self.node(DimensionExpression::Function(function, arguments))
        } else {
            self.node(DimensionExpression::Variable(name))
        }
    }
    fn identifier(&mut self) -> Result<String, IsfPrepareError> {
        self.skip_space();
        let start = self.position;
        if !self
            .source
            .get(self.position)
            .is_some_and(|value| value.is_ascii_alphabetic() || *value == b'_')
        {
            return Err(self.failure("expected a number, variable or function"));
        }
        self.position += 1;
        while self
            .source
            .get(self.position)
            .is_some_and(|value| value.is_ascii_alphanumeric() || *value == b'_')
        {
            self.position += 1;
        }
        Ok(String::from_utf8_lossy(&self.source[start..self.position]).into_owned())
    }
    fn number(&mut self) -> Result<DimensionExpression, IsfPrepareError> {
        let start = self.position;
        while self
            .source
            .get(self.position)
            .is_some_and(u8::is_ascii_digit)
        {
            self.position += 1;
        }
        if self.source.get(self.position) == Some(&b'.') {
            self.position += 1;
            while self
                .source
                .get(self.position)
                .is_some_and(u8::is_ascii_digit)
            {
                self.position += 1;
            }
        }
        if self
            .source
            .get(self.position)
            .is_some_and(|value| *value == b'e' || *value == b'E')
        {
            self.position += 1;
            if self
                .source
                .get(self.position)
                .is_some_and(|value| *value == b'+' || *value == b'-')
            {
                self.position += 1;
            }
            while self
                .source
                .get(self.position)
                .is_some_and(u8::is_ascii_digit)
            {
                self.position += 1;
            }
        }
        let token = std::str::from_utf8(&self.source[start..self.position])
            .map_err(|_| self.failure("number is not ASCII"))?;
        let value = token
            .parse::<f64>()
            .map_err(|_| self.failure("invalid numeric literal"))?;
        self.node(DimensionExpression::Number(finite_expression_value(value)?))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct IsfImportedImage {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Arc<[u8]>,
}

pub(crate) fn load_imported_images(
    imported: &Map<String, Value>,
    source_path: Option<&Path>,
) -> Result<Vec<IsfImportedImage>, IsfPrepareError> {
    if imported.is_empty() {
        return Ok(Vec::new());
    }
    if imported.len() > MAX_IMPORTED_IMAGES {
        return Err(error(format!(
            "ISF imported image count exceeds the {MAX_IMPORTED_IMAGES}-image safety limit"
        )));
    }
    let source_path = source_path.ok_or_else(|| error(
        "ISF imported image resources require the shader's source file path; import the shader from its .fs file together with its image assets",
    ))?;
    let source_directory = source_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let source_directory = source_directory.canonicalize().map_err(|io_error| {
        error(format!(
            "ISF imported image source directory cannot be resolved: {io_error}"
        ))
    })?;
    if !source_directory.is_dir() {
        return Err(error(
            "ISF imported image source directory is not a directory",
        ));
    }
    let mut result = Vec::with_capacity(imported.len());
    let mut total_rgba_bytes = 0_u64;
    for (name, resource) in imported {
        let path_text = resource
            .as_object()
            .and_then(|object| object.get("PATH"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                error(format!(
                    "ISF imported image '{name}' must be an object with a string PATH"
                ))
            })?;
        // Check both separators on every OS so accepted packs cannot become
        // traversal or device paths when moved to Windows.
        let normalized_path = path_text.replace('\\', "/");
        let relative_path = Path::new(&normalized_path);
        if normalized_path.trim().is_empty()
            || normalized_path.contains('\0')
            || normalized_path.contains(':')
            || relative_path.is_absolute()
            || relative_path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(error(format!(
                "ISF imported image '{name}' PATH must stay inside the shader directory; absolute paths, URLs and parent traversal are unsupported"
            )));
        }
        let resolved_path = source_directory
            .join(relative_path)
            .canonicalize()
            .map_err(|io_error| {
                error(format!(
                    "ISF imported image '{name}' cannot resolve PATH '{path_text}': {io_error}"
                ))
            })?;
        if !resolved_path.starts_with(&source_directory) {
            return Err(error(format!("ISF imported image '{name}' resolves outside the shader directory; symlink escapes are unsupported")));
        }
        let file = File::open(&resolved_path).map_err(|io_error| {
            error(format!(
                "ISF imported image '{name}' cannot be opened: {io_error}"
            ))
        })?;
        let metadata = file.metadata().map_err(|io_error| {
            error(format!(
                "ISF imported image '{name}' metadata cannot be read: {io_error}"
            ))
        })?;
        if !metadata.is_file() || metadata.len() > MAX_IMPORTED_FILE_BYTES {
            return Err(error(format!(
                "ISF imported image '{name}' must be a regular file no larger than 16 MiB"
            )));
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.take(MAX_IMPORTED_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|io_error| {
                error(format!(
                    "ISF imported image '{name}' cannot be read: {io_error}"
                ))
            })?;
        if bytes.len() as u64 > MAX_IMPORTED_FILE_BYTES {
            return Err(error(format!(
                "ISF imported image '{name}' exceeds the 16 MiB file safety limit"
            )));
        }
        let format = image::guess_format(&bytes).map_err(|decode_error| {
            error(format!(
            "ISF imported image '{name}' format cannot be read: {decode_error}; use PNG or JPEG"
        ))
        })?;
        if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
            return Err(error(format!(
                "ISF imported image '{name}' has an unsupported format; use PNG or JPEG"
            )));
        }
        let (width, height) = image::io::Reader::with_format(Cursor::new(bytes.as_slice()), format)
            .into_dimensions()
            .map_err(|decode_error| {
                error(format!(
                    "ISF imported image '{name}' dimensions cannot be decoded: {decode_error}"
                ))
            })?;
        let rgba_bytes = imported_rgba_size(width, height)?;
        total_rgba_bytes = total_rgba_bytes.checked_add(rgba_bytes).filter(|total| *total <= MAX_IMPORTED_RGBA_BYTES)
            .ok_or_else(|| error("ISF imported image resources exceed the combined 64 MiB decoded RGBA safety limit; reduce image sizes or the number of imports"))?;
        let mut limits = image::io::Limits::default();
        limits.max_image_width = Some(MAX_IMPORTED_DIMENSION);
        limits.max_image_height = Some(MAX_IMPORTED_DIMENSION);
        limits.max_alloc = Some(MAX_IMPORTED_RGBA_BYTES);
        let mut reader = image::io::Reader::with_format(Cursor::new(bytes), format);
        reader.limits(limits);
        let decoded = reader.decode().map_err(|decode_error| {
            error(format!(
            "ISF imported image '{name}' cannot be decoded within its safety limits: {decode_error}"
        ))
        })?;
        if decoded.width() != width || decoded.height() != height {
            return Err(error(format!(
                "ISF imported image '{name}' changed dimensions while decoding"
            )));
        }
        let rgba = decoded.into_rgba8().into_raw();
        if rgba.len() as u64 != rgba_bytes {
            return Err(error(format!(
                "ISF imported image '{name}' produced an unexpected RGBA byte count"
            )));
        }
        result.push(IsfImportedImage {
            name: name.clone(),
            width,
            height,
            rgba: Arc::from(rgba),
        });
    }
    Ok(result)
}

fn imported_rgba_size(width: u32, height: u32) -> Result<u64, IsfPrepareError> {
    if width == 0
        || height == 0
        || width > MAX_IMPORTED_DIMENSION
        || height > MAX_IMPORTED_DIMENSION
    {
        return Err(error(format!("ISF imported image dimensions must be between 1 and {MAX_IMPORTED_DIMENSION} pixels on each axis")));
    }
    let bytes = u64::from(width) * u64::from(height) * 4;
    if bytes > MAX_IMPORTED_RGBA_BYTES {
        return Err(error("ISF imported image exceeds the 64 MiB decoded RGBA safety limit; reduce the image dimensions"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn dimension(source: &str) -> IsfDimension {
        IsfDimension::parse(Some(&json!(source))).unwrap().unwrap()
    }
    fn scalar_definition(name: &str) -> IsfControlDefinition {
        IsfControlDefinition {
            name: name.to_owned(),
            kind: IsfControlKind::Float,
            default: [2.0, 0.0, 0.0, 0.0],
            minimum: [0.0; 4],
            maximum: [100.0; 4],
            labels: Vec::new(),
            values: Vec::new(),
        }
    }

    #[test]
    fn isf_dimensions_support_constants_arithmetic_and_frame_variables() {
        assert!(IsfDimension::parse(None).unwrap().is_none());
        assert_eq!(
            IsfDimension::parse(Some(&json!(127.9)))
                .unwrap()
                .unwrap()
                .evaluate(640, 480, &[], &[])
                .unwrap(),
            127
        );
        for (source, expected) in [
            ("$WIDTH / 2", 320),
            ("$HEIGHT * .25", 120),
            ("(2 + 3) * 4 - 1", 19),
            ("1e2 + 10 % 4", 102),
            ("2 ^ 3 ^ 2", 512),
            ("-2 ^ 2 + 8", 4),
            ("+4", 4),
        ] {
            assert_eq!(
                dimension(source).evaluate(640, 480, &[], &[]).unwrap(),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn isf_dimensions_support_common_functions_and_live_scalar_values() {
        for (source, expected) in [
            ("min($WIDTH, $HEIGHT)", 480),
            ("max(3, floor(4.9))", 4),
            ("ceil(4.1) + abs(-2)", 7),
            ("round(4.6) + trunc(4.9)", 9),
            ("pow(2, 4) + sqrt(9)", 19),
            ("clamp(20, 1, 10)", 10),
        ] {
            assert_eq!(
                dimension(source).evaluate(640, 480, &[], &[]).unwrap(),
                expected,
                "{source}"
            );
        }
        let definitions = [scalar_definition("scale")];
        let expression = dimension("$WIDTH / $scale");
        assert_eq!(
            expression.evaluate(640, 480, &definitions, &[]).unwrap(),
            320
        );
        assert_eq!(
            expression
                .evaluate(640, 480, &definitions, &[[4.0, 0.0, 0.0, 0.0]])
                .unwrap(),
            160
        );
    }

    #[test]
    fn isf_dimensions_reject_invalid_syntax_and_unbounded_expressions() {
        for source in [
            "",
            " ",
            "1 +",
            "()",
            "1 trailing",
            "1e+",
            ".",
            "min(1)",
            "max(1,2,3)",
            "floor()",
            "unknown(1)",
            "$WIDTH.x",
            "1; 2",
            "λ",
        ] {
            assert!(
                IsfDimension::parse(Some(&json!(source))).is_err(),
                "{source}"
            );
        }
        let too_deep = format!("{}1{}", "(".repeat(40), ")".repeat(40));
        assert!(IsfDimension::parse(Some(&json!(too_deep))).is_err());
        assert!(IsfDimension::parse(Some(&json!(vec!["1"; 200].join("+")))).is_err());
        assert!(IsfDimension::parse(Some(&json!("1".repeat(MAX_EXPRESSION_BYTES + 1)))).is_err());
        for value in [json!(null), json!(false), json!([]), json!({}), json!(0)] {
            assert!(IsfDimension::parse(Some(&value)).is_err());
        }
    }

    #[test]
    fn isf_dimensions_reject_invalid_results_and_non_scalar_inputs() {
        for source in [
            "0",
            "-1",
            ".5",
            "1 / 0",
            "3 % 0",
            "pow(1e200, 4)",
            "sqrt(-1)",
            "clamp(2, 4, 1)",
            "4294967296",
            "$missing",
        ] {
            assert!(
                dimension(source).evaluate(640, 480, &[], &[]).is_err(),
                "{source}"
            );
        }
        let mut vector = scalar_definition("vector");
        vector.kind = IsfControlKind::Point2d;
        assert!(dimension("$vector")
            .evaluate(640, 480, &[vector], &[])
            .is_err());
        assert!(dimension("$scale")
            .evaluate(
                640,
                480,
                &[scalar_definition("scale")],
                &[[f32::NAN, 0.0, 0.0, 0.0]]
            )
            .is_err());
    }

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let name = format!(
                "{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            );
            let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/t23-isf-resources")
                .join(name);
            std::fs::create_dir_all(&directory).unwrap();
            Self(directory)
        }
        fn shader_path(&self) -> PathBuf {
            self.0.join("shader.fs")
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn imports(path: &str) -> Map<String, Value> {
        json!({ "testImage": { "PATH": path } })
            .as_object()
            .unwrap()
            .clone()
    }

    #[test]
    fn isf_imports_decode_known_png_pixels() {
        let directory = TestDirectory::new();
        image::RgbaImage::from_raw(2, 1, vec![255, 0, 0, 255, 0, 128, 255, 64])
            .unwrap()
            .save_with_format(directory.0.join("pixels.png"), ImageFormat::Png)
            .unwrap();
        let loaded =
            load_imported_images(&imports("pixels.png"), Some(&directory.shader_path())).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "testImage");
        assert_eq!((loaded[0].width, loaded[0].height), (2, 1));
        assert_eq!(loaded[0].rgba.as_ref(), &[255, 0, 0, 255, 0, 128, 255, 64]);
    }

    #[test]
    fn isf_imports_decode_jpeg_and_nested_relative_paths() {
        let directory = TestDirectory::new();
        std::fs::create_dir(directory.0.join("assets")).unwrap();
        image::RgbImage::from_pixel(2, 1, image::Rgb([64, 128, 192]))
            .save_with_format(directory.0.join("assets/pixels.jpg"), ImageFormat::Jpeg)
            .unwrap();
        let loaded = load_imported_images(
            &imports("assets\\pixels.jpg"),
            Some(&directory.shader_path()),
        )
        .unwrap();
        assert_eq!((loaded[0].width, loaded[0].height), (2, 1));
        assert_eq!(loaded[0].rgba.len(), 8);
        assert_eq!(loaded[0].rgba[3], 255);
        assert_eq!(loaded[0].rgba[7], 255);
        for (actual, expected) in loaded[0].rgba[..3].iter().zip([64_u8, 128, 192]) {
            assert!(i16::from(*actual).abs_diff(i16::from(expected)) <= 3);
        }
    }

    #[test]
    fn isf_imports_require_safe_relative_paths_and_source_context() {
        let directory = TestDirectory::new();
        assert!(load_imported_images(&Map::new(), None).unwrap().is_empty());
        assert!(load_imported_images(&imports("pixels.png"), None).is_err());
        for path in [
            "../pixels.png",
            "..\\pixels.png",
            "assets/../../pixels.png",
            "/pixels.png",
            "C:\\pixels.png",
            "\\\\server\\pixels.png",
            "https://example.com/pixels.png",
            "pixels.png:stream",
            "",
            " ",
            "missing.png",
        ] {
            assert!(
                load_imported_images(&imports(path), Some(&directory.shader_path())).is_err(),
                "{path}"
            );
        }
        for declaration in [
            json!({"testImage": "pixels.png"}),
            json!({"testImage": {}}),
            json!({"testImage": {"PATH": 1}}),
        ] {
            assert!(load_imported_images(
                declaration.as_object().unwrap(),
                Some(&directory.shader_path())
            )
            .is_err());
        }
    }

    #[test]
    fn isf_imports_reject_unsupported_formats_and_resource_limits() {
        let directory = TestDirectory::new();
        std::fs::write(directory.0.join("image.gif"), b"GIF89a").unwrap();
        assert!(
            load_imported_images(&imports("image.gif"), Some(&directory.shader_path())).is_err()
        );
        std::fs::write(directory.0.join("broken.png"), b"not an image").unwrap();
        assert!(
            load_imported_images(&imports("broken.png"), Some(&directory.shader_path())).is_err()
        );
        File::create(directory.0.join("oversized.png"))
            .unwrap()
            .set_len(MAX_IMPORTED_FILE_BYTES + 1)
            .unwrap();
        assert!(
            load_imported_images(&imports("oversized.png"), Some(&directory.shader_path()))
                .is_err()
        );
        assert!(imported_rgba_size(0, 1).is_err());
        assert!(imported_rgba_size(MAX_IMPORTED_DIMENSION + 1, 1).is_err());
        assert!(imported_rgba_size(8_192, 8_192).is_err());
        assert_eq!(
            imported_rgba_size(4_096, 4_096).unwrap(),
            MAX_IMPORTED_RGBA_BYTES
        );
        let too_many = (0..=MAX_IMPORTED_IMAGES)
            .map(|index| (format!("image{index}"), json!({"PATH": "x.png"})))
            .collect();
        assert!(load_imported_images(&too_many, Some(&directory.shader_path())).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn isf_imports_reject_symlinks_escaping_the_shader_directory() {
        let directory = TestDirectory::new();
        let origin = directory.0.join("origin");
        std::fs::create_dir(&origin).unwrap();
        image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]))
            .save_with_format(directory.0.join("outside.png"), ImageFormat::Png)
            .unwrap();
        std::os::unix::fs::symlink(directory.0.join("outside.png"), origin.join("escape.png"))
            .unwrap();
        assert!(
            load_imported_images(&imports("escape.png"), Some(&origin.join("shader.fs"))).is_err()
        );
    }
}
