import { VariableSolver } from "./ui/component/VariableSolver";
import "bootstrap/dist/css/bootstrap.min.css";

function App() {
  return (
    <div className="container">
      <h1>HiddenMate</h1>
      <p>覆面駒・透明駒協力詰ソルバー</p>
      <VariableSolver />
    </div>
  );
}

export default App;
