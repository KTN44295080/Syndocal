/*{"INPUTS":[{"NAME":"overlayImage","TYPE":"image"},{"NAME":"inputImage","TYPE":"image"}]}*/
void main() {
    gl_FragColor = mix(IMG_THIS_NORM_PIXEL(inputImage), IMG_THIS_NORM_PIXEL(overlayImage), 0.5);
}
