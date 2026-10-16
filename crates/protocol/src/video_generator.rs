use serde::{Deserialize, Serialize};

use crate::{VideoSourceKind, VideoSourceSummary};

/// Media-free RGBA8 sources. Integer parameters are the portable rendering
/// contract; adding this enum leaves all legacy source wire values unchanged.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub enum VideoGenerator {
    Solid {
        color: [u8; 4],
    },
    LinearGradient {
        color_a: [u8; 4],
        color_b: [u8; 4],
        /// Clockwise degrees: zero is left-to-right, 90 is top-to-bottom.
        angle_degrees: u16,
    },
    Noise {
        seed: u32,
        /// Lattice cells along the longest output dimension, 1..=256.
        scale: u16,
        /// Hundredths of one temporal lattice unit per second, 0..=1000.
        /// Zero produces a static frame. Time is sampled at 30 Hz.
        speed: u16,
        color_a: [u8; 4],
        color_b: [u8; 4],
    },
}

impl VideoGenerator {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::LinearGradient { angle_degrees, .. } if *angle_degrees > 359 => {
                Err("Generator gradient angle must be in 0..=359 degrees".to_string())
            }
            Self::Noise { scale, .. } if !(1..=256).contains(scale) => {
                Err("Generator noise scale must be in 1..=256 cells".to_string())
            }
            Self::Noise { speed, .. } if *speed > 1000 => {
                Err("Generator noise speed must be in 0..=1000".to_string())
            }
            _ => Ok(()),
        }
    }

    /// A generator is independent of media files, decoder hints and finite
    /// media metadata. Reject contradictory state instead of ignoring it.
    pub fn validate_source(&self, source: &VideoSourceSummary) -> Result<(), String> {
        self.validate()?;
        if source.kind != VideoSourceKind::Generator(*self) {
            return Err("Generator source parameters do not match its source kind".to_string());
        }
        if source.path.is_some() || source.codec.is_some() || source.metadata.is_some() {
            return Err(
                "Generator sources must not contain a path, codec or media metadata".to_string(),
            );
        }
        if source
            .name
            .as_ref()
            .is_some_and(|name| name.trim().is_empty())
        {
            return Err("Generator source name must not be empty".to_string());
        }
        Ok(())
    }
}

impl VideoSourceSummary {
    pub fn validate_generator_source(&self) -> Result<(), String> {
        match &self.kind {
            VideoSourceKind::Generator(generator) => generator.validate_source(self),
            _ => Ok(()),
        }
    }

    pub fn from_generator(generator: VideoGenerator) -> Result<Self, String> {
        generator.validate()?;
        Ok(Self {
            kind: VideoSourceKind::Generator(generator),
            path: None,
            name: None,
            codec: None,
            metadata: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize)]
    enum LegacyVideoSourceKind {
        File,
        Camera,
        ScreenCapture,
        Ndi,
        Spout,
        Syphon,
        StillImage,
    }

    #[test]
    fn generator_project_roundtrip_and_legacy_reader_rejection() {
        let generator = VideoGenerator::LinearGradient {
            color_a: [255, 0, 0, 255],
            color_b: [0, 0, 255, 127],
            angle_degrees: 359,
        };
        let source = VideoSourceSummary::from_generator(generator).unwrap();
        let mut project = crate::EngineSnapshot::default();
        project.video.media_assets.push(crate::MediaAssetSummary {
            id: 1,
            label: "Gradient".into(),
            source: source.clone(),
            content_hash: None,
            byte_size: None,
        });
        let wire = serde_json::to_value(&project).unwrap();
        let decoded: crate::EngineSnapshot = serde_json::from_value(wire).unwrap();
        assert_eq!(decoded, project);
        crate::validate_media_asset_catalog(&decoded.video).unwrap();
        let old_reader_error = serde_json::from_value::<LegacyVideoSourceKind>(
            serde_json::to_value(source.kind).unwrap(),
        )
        .unwrap_err();
        assert!(old_reader_error
            .to_string()
            .contains("unknown variant `Generator`"));
    }

    #[test]
    fn generator_wire_roundtrip_preserves_legacy_sources_and_rejects_unknown_fields() {
        for legacy in [
            "File",
            "Camera",
            "ScreenCapture",
            "Ndi",
            "Spout",
            "Syphon",
            "StillImage",
        ] {
            let json = format!("\"{legacy}\"");
            let kind: VideoSourceKind = serde_json::from_str(&json).unwrap();
            assert_eq!(serde_json::to_string(&kind).unwrap(), json);
        }
        let generator = VideoGenerator::Solid {
            color: [1, 2, 3, 255],
        };
        let source = VideoSourceSummary::from_generator(generator).unwrap();
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(
            json["kind"],
            serde_json::json!({"Generator":{"Solid":{"color":[1,2,3,255]}}})
        );
        assert_eq!(
            serde_json::from_value::<VideoSourceSummary>(json).unwrap(),
            source
        );
        assert!(serde_json::from_str::<VideoGenerator>(
            r#"{"Solid":{"color":[0,0,0,255],"ignored":1}}"#
        )
        .is_err());
        assert!(
            serde_json::from_str::<VideoSourceKind>(r#"{"Generator":{"Unknown":{}}}"#).is_err()
        );
    }

    #[test]
    fn generator_catalog_rejects_invalid_parameters_and_media_identity() {
        let valid = VideoGenerator::Noise {
            seed: u32::MAX,
            scale: 256,
            speed: 1000,
            color_a: [0; 4],
            color_b: [255; 4],
        };
        valid.validate().unwrap();
        let noise = |scale, speed| VideoGenerator::Noise {
            seed: 0,
            scale,
            speed,
            color_a: [0; 4],
            color_b: [255; 4],
        };
        for invalid in [
            noise(0, 100),
            noise(257, 100),
            noise(1, 1001),
            VideoGenerator::LinearGradient {
                angle_degrees: 360,
                color_a: [0; 4],
                color_b: [255; 4],
            },
        ] {
            assert!(invalid.validate().is_err());
        }
        let source = VideoSourceSummary::from_generator(valid).unwrap();
        let mut video = crate::VideoSnapshot::default();
        video.media_assets.push(crate::MediaAssetSummary {
            id: 1,
            label: "Noise".into(),
            source: source.clone(),
            content_hash: None,
            byte_size: None,
        });
        crate::validate_media_asset_catalog(&video).unwrap();
        video.media_assets[0].source.path = Some("unexpected.mp4".into());
        assert!(crate::validate_media_asset_catalog(&video)
            .unwrap_err()
            .contains("must not contain"));
        video.media_assets[0].source = source;
        video.media_assets[0].content_hash = Some(crate::MediaContentHash {
            algorithm: crate::MediaHashAlgorithm::Sha256,
            hex: "0".repeat(64),
        });
        video.media_assets[0].byte_size = Some(1);
        assert!(crate::validate_media_asset_catalog(&video).is_err());
    }
}
