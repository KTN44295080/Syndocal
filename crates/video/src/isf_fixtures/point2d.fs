/*{
  "ISFVSN":"2",
  "INPUTS":[
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"position","TYPE":"point2D","DEFAULT":[0.25,0.75],"MIN":[0.0,0.0],"MAX":[1.0,1.0]}
  ]
}*/
void main() { gl_FragColor = vec4(position, 0.0, 1.0); }
