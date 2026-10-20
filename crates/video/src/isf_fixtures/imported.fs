/*{
  "ISFVSN":"2",
  "INPUTS":[{"NAME":"inputImage","TYPE":"image"}],
  "IMPORTED":{"testImage":{"PATH":"two-pixels.png"}}
}*/
void main() {
    if (IMG_SIZE(testImage) != vec2(2.0, 1.0)) {
        gl_FragColor = vec4(0.0, 0.0, 1.0, 1.0);
    } else if (gl_FragCoord.x < 1.0) {
        gl_FragColor = IMG_PIXEL(testImage, vec2(1.5, 0.5));
    } else {
        gl_FragColor = IMG_NORM_PIXEL(testImage, vec2(0.25, 0.5));
    }
}
