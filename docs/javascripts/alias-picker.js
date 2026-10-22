// Clicking a row of the tables on the allowed_values page shows that row in the example above it.
// Alias rows update both the Alias and the Long form example; Long form rows update the Long form
// example. All data comes from the tables, so they stay the single source; the example values sit
// in a last column that CSS hides.

// Token classes Pygments uses for YAML, so rewritten examples keep their highlighting.
const INDICATOR = "p p-Indicator";
const PUNCTUATION = "p";
const KEY = "nt";
const SCALAR = "l l-Scalar l-Scalar-Plain";
const ITEM = "nv";
const SPACE = "w";
const LONG_FORM_TYPE = "allowed_values";
const LINE_ANCHOR = /^__codelineno-.*-/;
const KEY_VALUE = /:\s*/;
const ESCAPES = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" };

const escape = (text) => text.replace(/[&<>"]/g, (character) => ESCAPES[character]);
const span = (token, text) => `<span class="${token}">${escape(text)}</span>`;
const pair = (key, value) => span(KEY, key) + span(PUNCTUATION, ":") + span(SPACE, " ") + value;
const firstEntry = (key, value) => span(INDICATOR, "-") + span(SPACE, " ") + pair(key, value);
const nestedEntry = (key, value) => span(SPACE, "  ") + pair(key, value);
const list = (items) =>
  span(INDICATOR, "[") +
  items.map((item) => span(ITEM, item)).join(span(INDICATOR, ",") + span(SPACE, " ")) +
  span(INDICATOR, "]");

// A keyed field such as `customProperties: my_custom_property` is written as a flow mapping.
const fieldValue = (field) => {
  const [key, value] = field.split(KEY_VALUE);
  return value === undefined
    ? span(SCALAR, field)
    : span(INDICATOR, "{") + pair(key, span(ITEM, value)) + span(INDICATOR, "}");
};

// Keeps the line anchors Zensical adds to every code line.
const render = (code, lines) => {
  const prefix = code.querySelector("a[id^='__codelineno-']")?.id.match(LINE_ANCHOR)?.[0];
  const anchor = (number) =>
    prefix ? `<a id="${prefix}${number}" name="${prefix}${number}" href="#${prefix}${number}"></a>` : "";
  code.innerHTML = lines.map((line, index) => anchor(index + 1) + line).join("\n") + "\n";
};

const codesIn = (cell) => [...cell.querySelectorAll("code")].map((code) => code.textContent);
const valuesOf = (row) =>
  nestedEntry("values", list(row.cells[row.cells.length - 1].textContent.trim().split(/,\s*/)));

const longForm = (field, presets, values) => [
  firstEntry("type", span(SCALAR, LONG_FORM_TYPE)),
  nestedEntry("field", fieldValue(field)),
  ...presets.map((preset) => {
    const [key, value] = preset.split(KEY_VALUE);
    return nestedEntry(key, span(SCALAR, value));
  }),
  values,
];

const makeClickable = (rows, onSelect, isShown) => {
  const select = (row) => {
    onSelect(row);
    rows.forEach((other) => other.classList.toggle("selected", other === row));
  };
  rows.forEach((row) => {
    row.classList.add("alias-picker__row");
    row.classList.toggle("selected", isShown(row));
    row.tabIndex = 0;
    row.addEventListener("click", () => select(row));
    row.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        select(row);
      }
    });
  });
};

// `document$` fires on every page load, including instant navigation.
document$.subscribe(() => {
  document.querySelectorAll(".alias-picker").forEach((picker) => {
    const blocks = [...picker.querySelectorAll(".tabbed-block")];
    const [aliasCode, longCode] = blocks.map((block) => block.querySelector("pre code"));
    if (!aliasCode || !longCode) {
      return;
    }
    const [aliasRows, longRows] = blocks.map((block) => [...block.querySelectorAll("tbody tr")]);
    const shownAlias = aliasCode.textContent.match(/type: (\S+)/)?.[1];
    const shownField = longCode.textContent.match(/field: (\S+)/)?.[1];

    makeClickable(
      aliasRows,
      (row) => {
        const [alias] = codesIn(row.cells[0]);
        const [field, ...presets] = codesIn(row.cells[1]);
        const values = valuesOf(row);
        render(aliasCode, [firstEntry("type", span(SCALAR, alias)), values]);
        render(longCode, longForm(field, presets, values));
      },
      (row) => codesIn(row.cells[0])[0] === shownAlias,
    );
    makeClickable(
      longRows,
      (row) => render(longCode, longForm(codesIn(row.cells[0])[0], [], valuesOf(row))),
      (row) => codesIn(row.cells[0])[0] === shownField,
    );
  });
});
