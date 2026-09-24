//! ARC-005: a seam serves two. An item a `mod.rs` door offers past its parent,
//! that exactly one module outside the door uses and that no module inside
//! shares besides the one defining it, could live next to its caller: it is
//! refused unless `maestro-quality.toml` records why it stays. The crate root
//! composes the crate and is not counted as a caller.

use crate::checks::findings::{Finding, relative};
use crate::checks::module_tree::{Module, Tree};
use std::path::Path;

/// One thing a door offers past its parent.
struct Offer {
    /// The name a caller writes after the door.
    name: String,
    /// The line of the door offering it.
    line: usize,
    /// Its absolute path through the door.
    through: Vec<String>,
    /// Its absolute path where it is defined.
    defined: Vec<String>,
}

/// ARC-005 over one tree.
pub(super) fn findings(tree: &Tree, workspace: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    for door in tree.modules.iter().filter(|module| module.is_mod_rs()) {
        for offer in offers(tree, door) {
            let (outside, inside) = users(tree, door, &offer);
            let [only] = outside.as_slice() else {
                continue;
            };
            if !inside.is_empty() {
                continue;
            }
            let message = format!(
                "`{}` serves only {}; move it next to its caller, or record in \
                 maestro-quality.toml why this seam stays",
                offer.name,
                relative(workspace, &only.file)
            );
            found.push(
                Finding::new(
                    "ARC-005",
                    relative(workspace, &door.file),
                    offer.line,
                    message,
                )
                .about(&offer.name),
            );
        }
    }
    found
}

/// What a door offers past its parent: the modules it declares with a wide
/// visibility, and the names it re-exports with one.
fn offers(tree: &Tree, door: &Module) -> Vec<Offer> {
    let mut offers: Vec<Offer> = door
        .children()
        .filter(|child| child.is_offered())
        .map(|child| {
            let mut through = door.path.clone();
            through.push(child.name.clone());
            Offer {
                name: child.name.clone(),
                line: child.line,
                defined: through.clone(),
                through,
            }
        })
        .collect();
    for export in door.exports.iter().filter(|export| export.offered) {
        let Some(defined) = tree.absolute(&door.path, &export.segments) else {
            continue;
        };
        let Some(name) = defined.last().cloned() else {
            continue;
        };
        let mut through = door.path.clone();
        through.push(name.clone());
        offers.push(Offer {
            name,
            line: export.line,
            through,
            defined,
        });
    }
    offers
}

/// The modules outside the door that use an offer, the crate root left out,
/// and those inside that use it besides the door and the defining module.
fn users<'a>(tree: &'a Tree, door: &Module, offer: &Offer) -> (Vec<&'a Module>, Vec<&'a Module>) {
    let defining = tree
        .landing(&offer.defined)
        .and_then(|index| tree.modules.get(index))
        .map(|module| module.path.clone())
        .unwrap_or_default();
    let mut outside = Vec::new();
    let mut inside = Vec::new();
    for module in &tree.modules {
        let skipped = module.path.is_empty()
            || module.path == door.path
            || module.path.starts_with(&defining);
        if skipped || !uses(tree, module, offer) {
            continue;
        }
        if module.path.starts_with(&door.path) {
            inside.push(module);
        } else {
            outside.push(module);
        }
    }
    (outside, inside)
}

/// Whether a module names an offer, through the door or where it is defined.
fn uses(tree: &Tree, module: &Module, offer: &Offer) -> bool {
    module
        .paths
        .iter()
        .filter_map(|path| tree.absolute(&module.path, &path.segments))
        .any(|absolute| {
            absolute.starts_with(&offer.through) || absolute.starts_with(&offer.defined)
        })
}

#[cfg(test)]
mod tests {
    use super::findings;
    use crate::checks::module_tree::sample;
    use std::path::Path;

    #[test]
    fn a_seam_with_one_outside_caller_and_no_inside_sharer_is_refused() {
        let tree = sample(
            &["bin"],
            &[
                (
                    "main.rs",
                    "mod first;\nmod runner;\nmod second;\nfn main() { runner::root_only(); }\n",
                ),
                ("first.rs", "use crate::runner::{Cmd, only, shared};\n"),
                ("second.rs", "use crate::runner::Cmd;\n"),
                (
                    "runner/mod.rs",
                    concat!(
                        "mod commands;\nmod jobs;\n",
                        "pub(crate) use commands::{Cmd, only, root_only, shared};\n",
                        "pub(super) use commands::narrow;\n",
                    ),
                ),
                ("runner/commands.rs", "pub(crate) struct Cmd;\n"),
                ("runner/jobs.rs", "use super::commands::shared;\n"),
            ],
        );
        let found = findings(&tree, Path::new("/w"));
        let lines: Vec<String> = found.iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            [
                "ARC-005 src/runner/mod.rs:3: `only` serves only src/first.rs; move it next to \
              its caller, or record in maestro-quality.toml why this seam stays"
            ]
        );
        assert_eq!(found[0].item, "only");
    }
}
