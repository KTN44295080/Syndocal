/*{
  "ISFVSN":"2",
  "INPUTS":[{"NAME":"inputImage","TYPE":"image"}],
  "PASSES":[
    {"TARGET":"halfBuffer","WIDTH":"$WIDTH / 2","HEIGHT":"$HEIGHT / 2"},
    {}
  ]
}*/
void main() {
    if (PASSINDEX == 0) {
        gl_FragColor = vec4(RENDERSIZE.x / 8.0, RENDERSIZE.y / 4.0, 0.0, 1.0);
    } else {
        gl_FragColor = vec4(IMG_PIXEL(halfBuffer, vec2(0.5)).r,
            IMG_SIZE(halfBuffer).y / 4.0, float(PASSINDEX), 1.0);
    }
}
