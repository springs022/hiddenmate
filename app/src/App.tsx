import { useState } from "react";
import { Editor } from "./ui/component/Editor";
import "bootstrap/dist/css/bootstrap.min.css";

function App() {
  return (
    <div className="container">
      <h1>HiddenMate</h1>
      <p>覆面駒・透明駒協力詰ソルバー（通常協力詰UI）</p>
      <Editor />
    </div>
  );
}

export default App;
