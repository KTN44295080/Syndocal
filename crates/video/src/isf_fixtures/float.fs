/*{
  "ISFVSN":"2",
  "INPUTS":[{"NAME":"inputImage","TYPE":"image"}],
  "PASSES":[{"TARGET":"floatBuffer","FLOAT":true},{}]
}*/
void main() {
    if (PASSINDEX == 0) {
        gl_FragColor = vec4(2049.0, -2.0, 2.0, 1.0);
    } else {
        vec4 value = IMG_THIS_PIXEL(floatBuffer);
        gl_FragColor = vec4(value.r - 2048.0, (value.g + 4.0) / 4.0, value.b / 4.0, 1.0);
    }
}
