// Clicking a row of an alias table rewrites the Alias and Long form examples above it. All data
// comes from the row itself (alias, field path, optional `role: ...`, example values), so the
// table stays the single source.
// `document$` fires on every page load, including instant navigation.
document$.subscribe(() => {
  document.querySelectorAll(".alias-picker").forEach((picker) => {
    const blocks = [...picker.querySelectorAll(".tabbed-block")];
    const [aliasCode, longCode] = blocks.map((block) => block.querySelector("pre code"));
    if (!aliasCode || !longCode) {
      return;
    }
    const rows = [...blocks[0].querySelectorAll("tbody tr")];
    const aliasOf = (row) => row.cells[0].querySelector("code")?.textContent ?? "";

    const select = (row) => {
      const [field, ...presets] = [...row.cells[1].querySelectorAll("code")].map(
        (code) => code.textContent,
      );
      const values = `  values: [${row.cells[2].textContent.trim()}]`;
      aliasCode.textContent = [`- type: ${aliasOf(row)}`, values, ""].join("\n");
      longCode.textContent = [
        "- type: allowed_values",
        `  field: ${field}`,
        ...presets.map((preset) => `  ${preset}`),
        values,
        "",
      ].join("\n");
      rows.forEach((other) => other.classList.toggle("selected", other === row));
    };

    const shown = aliasCode.textContent.match(/type: (\S+)/)?.[1];
    rows.forEach((row) => {
      row.classList.add("alias-picker__row");
      row.classList.toggle("selected", aliasOf(row) === shown);
      row.tabIndex = 0;
      row.addEventListener("click", () => select(row));
      row.addEventListener("keydown", (event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          select(row);
        }
      });
    });
  });
});
