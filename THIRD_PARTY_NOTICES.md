# Third-party notices

HiddenMateは、以下の第三者ソフトウェアを利用しています。この一覧は、ソースコードおよびWeb配布物に直接含まれる主要なランタイム依存関係を対象としています。

## fmrs

HiddenMateは、Keigo Oka氏による将棋協力詰ソルバー [fmrs](https://github.com/ogiekako/fmrs) を基にした派生ソフトウェアであり、fmrsの実質的な部分を含みます。

- License: MIT License
- Copyright: Copyright (c) 2025 Keigo Oka

fmrsの著作権表示とMIT License全文は [LICENSE](LICENSE) に保持しています。HiddenMateで追加した変更部分も同じMIT Licenseで提供します。

## Web版の主要なランタイム依存関係

| ソフトウェア                        | ライセンス                                    | 出典                                               |
| ----------------------------------- | --------------------------------------------- | -------------------------------------------------- |
| React / React DOM / Scheduler       | MIT                                           | https://github.com/facebook/react                  |
| Bootstrap                           | MIT                                           | https://github.com/twbs/bootstrap                  |
| React Bootstrap                     | MIT                                           | https://github.com/react-bootstrap/react-bootstrap |
| react-icons                         | MIT（アイコンは各収録元のライセンスにも従う） | https://github.com/react-icons/react-icons         |
| Bootstrap Icons（`react-icons/bs`） | MIT                                           | https://github.com/twbs/icons                      |
| web-vitals                          | Apache License 2.0                            | https://github.com/GoogleChrome/web-vitals         |
| classnames                          | MIT                                           | https://github.com/JedWatson/classnames            |

Web版のビルドでは、npmから実際に取り込んだ主要なランタイム依存関係のライセンス原文を `third-party-licenses/` に出力します。Webpackが生成する `main.js.LICENSE.txt` にも、バンドル時に検出された著作権表示が保存されます。

## Rust / WebAssembly依存関係

WebAssemblyおよびCLIは、各 `Cargo.toml` に記載されたRust crateを利用します。各crateにはそれぞれのライセンスが適用されます。配布時に使用した正確な依存関係はCargoのビルド情報を基準とし、第三者crateの著作権・ライセンス表示を削除しないでください。

本ファイルは各第三者ライセンスを変更するものではありません。各ソフトウェアには、それぞれのライセンス本文が優先して適用されます。
