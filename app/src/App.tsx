import "bootstrap/dist/css/bootstrap.min.css";
import { Card } from "react-bootstrap";
import { VariableSolver } from "./ui/component/VariableSolver";

declare const FMRS_BASE_PATH: string;

function App() {
  return (
    <div className="container">
      <h1>HiddenMate</h1>
      <p>
        覆面駒・透明駒の検討{" "}
        <span className="badge bg-secondary">覆面駒版 β</span>
      </p>
      <VariableSolver />
      <Card className="mb-4">
        <Card.Header as="h2" className="h5 mb-0">
          透明駒
        </Card.Header>
        <Card.Body>
          <p className="mb-0">透明駒の検討機能は開発中です。</p>
        </Card.Body>
      </Card>
      <footer className="border-top mt-4 py-3 small text-secondary">
        HiddenMateは
        <a
          href="https://github.com/ogiekako/fmrs"
          target="_blank"
          rel="noopener noreferrer"
        >
          fmrs
        </a>
        を基にしたMIT Licenseのソフトウェアです。{" "}
        <a href={`${FMRS_BASE_PATH}LICENSE`}>ライセンス</a>
        {" / "}
        <a href={`${FMRS_BASE_PATH}THIRD_PARTY_NOTICES.md`}>第三者ライセンス</a>
      </footer>
    </div>
  );
}

export default App;
