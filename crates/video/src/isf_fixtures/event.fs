/*{
  "ISFVSN":"2",
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"trigger","TYPE":"event"}
  ],
  "PASSES":[{"TARGET":"eventBuffer"},{}]
}*/
void main() {
    if (PASSINDEX == 0) {
        gl_FragColor = vec4(trigger ? 1.0 : 0.0, 0.0, 0.0, 1.0);
    } else {
        gl_FragColor = IMG_THIS_PIXEL(eventBuffer);
    }
}
