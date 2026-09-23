# One of the README's three gate tables from docs/gates.toml; $kind is always,
# default or opt-in. Read by `just docs`.
def proofs: [.proofs[] | "`\(.)`"] | join(", ");
[.gate[] | select(.kind == $kind)]
| if $kind == "always" then
    "| Gate | What fails the run | Standard | Proof |", "| --- | --- | --- | --- |",
    (.[] | "| \(.name) | \(.fails) | \(.standard) | \(proofs) |")
  elif $kind == "default" then
    "| Gate | Input | What fails the run | Standard | Proof |",
    "| --- | --- | --- | --- | --- |",
    (.[] | "| \(.name) | \(.switch) | \(.fails) | \(.standard) | \(proofs) |")
  else
    "| Gate | How | Standard | Proof |", "| --- | --- | --- | --- |",
    (.[] | "| \(.name) | \(.switch) | \(.standard) | \(proofs) |")
  end
