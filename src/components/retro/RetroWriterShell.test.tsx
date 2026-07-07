import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, test, vi } from "vitest";

import { RetroWriterShell } from "./RetroWriterShell";

describe("RetroWriterShell", () => {
  test("renders a selected retro theme and forwards textarea edits", () => {
    const onChange = vi.fn();

    render(<RetroWriterShell text="Hello old screen" themeId="amber-ruler" onChange={onChange} />);

    expect(screen.getByLabelText("Retro Writer in Amber Ruler theme")).toBeInTheDocument();

    const editor = screen.getByRole("textbox");
    fireEvent.change(editor, { target: { value: "New amber words" } });

    expect(onChange).toHaveBeenCalledWith("New amber words");
  });
});
