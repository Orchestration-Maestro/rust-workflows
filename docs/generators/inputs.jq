# A workflow's inputs as a Markdown table. $columns picks the shape:
# "type-default", "default" or "required". Read by `just docs`.
def default:
  if .required then "required"
  elif .default == null or .default == "" then "Empty"
  else "`\(.default)`" end;
def cell: gsub("\n"; " ");
.on.workflow_call.inputs // {} | to_entries
| if $columns == "type-default" then
    "| Input | Type | Default | Meaning |", "| --- | --- | --- | --- |",
    (.[] | "| `\(.key)` | \(.value.type) | \(.value | default) | \(.value.description | cell) |")
  elif $columns == "required" then
    "| Input | Required | Meaning |", "| --- | --- | --- |",
    (.[] | "| `\(.key)` | \(if .value.required then "yes" else "no" end) | \(.value.description | cell) |")
  else
    "| Input | Default | Meaning |", "| --- | --- | --- |",
    (.[] | "| `\(.key)` | \(.value | default) | \(.value.description | cell) |")
  end
