import { VariableSolver } from "./ui/component/VariableSolver";
import "bootstrap/dist/css/bootstrap.min.css";

declare const FMRS_BASE_PATH: string;

function App() {
  return (
    <div className="container">
      <h1>HiddenMate</h1>
      <p>覆面駒・透明駒協力詰ソルバー</p>
      <VariableSolver />
      <footer className="border-top mt-4 py-3 small text-secondary">
        HiddenMateはfmrsを基にしたMIT Licenseのソフトウェアです。{" "}
        <a href={`${FMRS_BASE_PATH}LICENSE`}>ライセンス</a>
        {" / "}
        <a href={`${FMRS_BASE_PATH}THIRD_PARTY_NOTICES.md`}>第三者ライセンス</a>
      </footer>
    </div>
  );
}

export default App;
