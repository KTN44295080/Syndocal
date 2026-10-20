/*{
  "ISFVSN":"2",
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"audioWave","TYPE":"audio","MAX":512},
    {"NAME":"audioSpectrum","TYPE":"audioFFT","MAX":512}
  ]
}*/
void main() {
    gl_FragColor = vec4(
        IMG_NORM_PIXEL(audioWave, vec2(0.0, 0.5)).r,
        IMG_NORM_PIXEL(audioSpectrum, vec2(0.0, 0.5)).r,
        IMG_SIZE(audioWave).x / 512.0,
        1.0);
}
