import React from "react";
import { render, screen } from "@testing-library/react";
import App from "./App";

test("renders HiddenMate title", () => {
  render(<App />);
  expect(screen.getByRole("heading", { name: "HiddenMate" })).not.toBeNull();
  expect(
    screen.getByRole("heading", { name: "覆面駒（Variable）検討" })
  ).not.toBeNull();
  expect(screen.getByRole("button", { name: "盤面・フォーム" })).not.toBeNull();
  expect(screen.getByLabelText("通常駒のbase SFEN")).not.toBeNull();
  expect(screen.getByText("受方駒台（自動補完）")).not.toBeNull();
  expect(screen.getByText("攻方駒台")).not.toBeNull();
  expect(screen.getByRole("button", { name: "＋△覆面駒" })).not.toBeNull();
  expect(screen.getByRole("button", { name: "＋▲覆面駒" })).not.toBeNull();
  expect(screen.getByRole("button", { name: "覆面駒を検討" })).not.toBeNull();
});
