/*{"PASSES":[{"TARGET":"dateBuffer"},{}]}*/
void main() {
    if (PASSINDEX == 0) {
        gl_FragColor = vec4((DATE.x - 2000.0) / 255.0, DATE.y / 255.0, DATE.z / 255.0, DATE.w - 43200.0);
    } else {
        gl_FragColor = IMG_THIS_NORM_PIXEL(dateBuffer);
    }
}
