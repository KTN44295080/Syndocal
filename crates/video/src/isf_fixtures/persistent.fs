/*{
  "ISFVSN":"2",
  "INPUTS":[{"NAME":"inputImage","TYPE":"image"}],
  "PASSES":[{"TARGET":"historyBuffer","PERSISTENT":true},{}]
}*/
void main() {
    if (PASSINDEX == 0) {
        gl_FragColor = vec4(IMG_THIS_NORM_PIXEL(historyBuffer).r + 1.0 / 255.0, 0.0, 0.0, 1.0);
    } else {
        gl_FragColor = IMG_THIS_PIXEL(historyBuffer);
    }
}
