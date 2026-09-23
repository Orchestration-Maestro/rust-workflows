# A workflow's outputs as a Markdown table. Read by `just docs`.
"| Output | Meaning |", "| --- | --- |",
(.on.workflow_call.outputs // {} | to_entries[] | "| `\(.key)` | \(.value.description | gsub("\n"; " ")) |")
