# The inputs this workflow has and $other, a second one as JSON, lacks, as a
# Markdown table. Read by `just docs`.
def default:
  if .required then "required"
  elif .default == null or .default == "" then "Empty"
  else "`\(.default)`" end;
def cell: gsub("\n"; " ");
$other[0].on.workflow_call.inputs as $second
| "| Input | Type | Default | Meaning |", "| --- | --- | --- | --- |",
  (.on.workflow_call.inputs | to_entries[] | select($second[.key] == null)
    | "| `\(.key)` | \(.value.type) | \(.value | default) | \(.value.description | cell) |")
