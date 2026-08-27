//! Immutable parent and sibling navigation for one published tree.

use anodrel_windows_accessibility::AccessibleElement;

use crate::raw2::direction;

/// Immutable direct relationships derived from one mapped semantic snapshot.
///
/// The portable model emits preorder data, so a valid parent is earlier than
/// its child. Rejecting every other value keeps an accidentally malformed
/// mapping bounded and acyclic; it becomes a top-level child rather than a
/// relationship the provider cannot safely navigate.
#[derive(Debug)]
pub(super) struct Relationships {
    parents: Vec<Option<usize>>,
    children: Vec<Vec<usize>>,
    root_children: Vec<usize>,
}

impl Relationships {
    pub(super) fn from_elements(elements: &[AccessibleElement]) -> Self {
        let mut relationships = Self {
            parents: vec![None; elements.len()],
            children: (0..elements.len()).map(|_| Vec::new()).collect(),
            root_children: Vec::new(),
        };

        for (index, element) in elements.iter().enumerate() {
            let parent = element
                .parent_index()
                .filter(|parent| *parent < index && *parent < elements.len());
            relationships.parents[index] = parent;
            match parent {
                Some(parent) => relationships.children[parent].push(index),
                None => relationships.root_children.push(index),
            }
        }
        relationships
    }

    pub(super) fn step(&self, element: Option<usize>, towards: i32) -> Option<Option<usize>> {
        match element {
            // The root's parent belongs to Windows, not to this provider.
            None => match towards {
                direction::PARENT => None,
                direction::FIRST_CHILD => self.root_children.first().copied().map(Some),
                direction::LAST_CHILD => self.root_children.last().copied().map(Some),
                _ => None,
            },
            Some(index) => {
                let parent = *self.parents.get(index)?;
                match towards {
                    direction::PARENT => Some(parent),
                    direction::FIRST_CHILD => self.children[index].first().copied().map(Some),
                    direction::LAST_CHILD => self.children[index].last().copied().map(Some),
                    direction::NEXT_SIBLING => {
                        let siblings = self.siblings(index, parent)?;
                        let position = siblings.iter().position(|sibling| *sibling == index)?;
                        siblings.get(position + 1).copied().map(Some)
                    }
                    direction::PREVIOUS_SIBLING => {
                        let siblings = self.siblings(index, parent)?;
                        let position = siblings.iter().position(|sibling| *sibling == index)?;
                        position
                            .checked_sub(1)
                            .and_then(|position| siblings.get(position))
                            .copied()
                            .map(Some)
                    }
                    _ => None,
                }
            }
        }
    }

    fn siblings(&self, index: usize, parent: Option<usize>) -> Option<&[usize]> {
        self.parents.get(index)?;
        match parent {
            Some(parent) => self.children.get(parent).map(Vec::as_slice),
            None => Some(&self.root_children),
        }
    }
}
