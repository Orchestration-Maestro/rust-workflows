# The inputs two publishers share, from the first workflow and $other, the second as JSON: a
# default that differs is shown as "first / second", and every input
# forwarded to ci.yml shares one row. Read by `just docs`.
def default:
  if .required then "required"
  elif .default == null or .default == "" then "Empty"
  else "`\(.default)`" end;
def cell: gsub("\n"; " ");
.on.workflow_call.inputs as $first
| $other[0].on.workflow_call.inputs as $second
| [$first | to_entries[] | select($second[.key] != null)] as $shared
| [$shared[] | select(.value.description | startswith("Forwarded"))] as $forwarded
| "| Input | Type | Default | Meaning |", "| --- | --- | --- | --- |",
  ($shared[] | select(.value.description | startswith("Forwarded") | not)
    | ($second[.key] | default) as $other
    | (.value | default) as $mine
    | "| `\(.key)` | \(.value.type) | \(if $mine == $other then $mine else "\($mine) / \($other)" end) | \(.value.description | cell) |"),
  "| \([$forwarded[].key | "`\(.)`"] | join(", ")) | as in [ci.md](ci.md) | as in `ci.yml` | \($forwarded[0].value.description | cell) |"
