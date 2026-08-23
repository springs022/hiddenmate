import React from "react";
import { render, screen } from "@testing-library/react";
import App from "./App";

test("renders HiddenMate title", () => {
  render(<App />);
  expect(screen.getByRole("heading", { name: "HiddenMate" })).not.toBeNull();
});
