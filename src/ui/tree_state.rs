use crate::entries::{EntryStatus, ListEntry};
use std::path::PathBuf;

/// A node in the file tree
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub entry: ListEntry,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
    pub depth: usize,
}

impl TreeNode {
    pub fn new(entry: ListEntry, depth: usize) -> Self {
        Self {
            entry,
            children: Vec::new(),
            expanded: false,
            depth,
        }
    }

    pub fn new_expanded(entry: ListEntry, depth: usize) -> Self {
        Self {
            entry,
            children: Vec::new(),
            expanded: true,
            depth,
        }
    }

    /// Count selected files in this node and all children
    pub fn count_selected(&self) -> usize {
        let mut count = if self.entry.selected && !self.entry.is_directory {
            1
        } else {
            0
        };
        for child in &self.children {
            count += child.count_selected();
        }
        count
    }

    /// Find a node by path, searching this node and its descendants
    pub fn find(&self, path: &str) -> Option<&TreeNode> {
        if self.entry.relative_path == path {
            return Some(self);
        }

        for child in &self.children {
            if let Some(found) = child.find(path) {
                return Some(found);
            }
        }

        None
    }

    /// Find a node by path, searching this node and its descendants
    pub fn find_mut(&mut self, path: &str) -> Option<&mut TreeNode> {
        if self.entry.relative_path == path {
            return Some(self);
        }

        for child in &mut self.children {
            if let Some(found) = child.find_mut(path) {
                return Some(found);
            }
        }

        None
    }

    /// Find and update an entry's status by path
    pub fn update_entry_status(&mut self, path: &str, status: EntryStatus) -> bool {
        match self.find_mut(path) {
            Some(node) => {
                node.entry.status = status;
                true
            }
            None => false,
        }
    }

    /// Toggle expanded state
    pub fn toggle_expanded(&mut self) {
        if self.entry.is_directory {
            self.expanded = !self.expanded;
        }
    }

    /// Expand this node and all parents to a specific path
    pub fn expand_path_to(&mut self, target_path: &str) -> bool {
        // If this is the target, we're done
        if self.entry.relative_path == target_path {
            return true;
        }

        // If the target path starts with this node's path, search children
        if target_path.starts_with(&self.entry.relative_path) || self.entry.relative_path == "." {
            for child in &mut self.children {
                if child.expand_path_to(target_path) {
                    // Found it in a child, so expand this node
                    if self.entry.is_directory {
                        self.expanded = true;
                    }
                    return true;
                }
            }
        }

        false
    }

    /// Flatten tree to visible rows (respecting expanded state)
    pub fn flatten(&self, result: &mut Vec<FlattenedNode>) {
        result.push(FlattenedNode {
            entry: self.entry.clone(),
            depth: self.depth,
            expanded: self.expanded,
            has_children: !self.children.is_empty(),
            selected_count: if self.entry.is_directory {
                Some(self.count_selected())
            } else {
                None
            },
        });

        // Show children only if:
        // - This is a directory AND it's expanded
        // - OR if this is the root (depth 0) which should always show its children
        if (self.entry.is_directory && self.expanded) || self.depth == 0 {
            for child in &self.children {
                child.flatten(result);
            }
        }
    }
}

/// A flattened representation of a tree node for rendering
#[derive(Debug, Clone)]
pub struct FlattenedNode {
    pub entry: ListEntry,
    pub depth: usize,
    pub expanded: bool,
    pub has_children: bool,
    pub selected_count: Option<usize>,
}

/// State for tree view navigation
pub struct TreeState {
    root: TreeNode,
    cursor: usize,
    flattened_cache: Vec<FlattenedNode>,
    cache_dirty: bool,
}

impl TreeState {
    pub fn new(base_path: PathBuf) -> Self {
        let root_entry = ListEntry {
            name: base_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("root")
                .to_string(),
            relative_path: String::from("."),
            selected: false,
            is_directory: true,
            status: EntryStatus::Unknown,
        };

        Self {
            root: TreeNode::new_expanded(root_entry, 0),
            cursor: 0,
            flattened_cache: Vec::new(),
            cache_dirty: true,
        }
    }

    /// Build tree from flat list of entries
    pub fn build_from_entries(&mut self, entries: Vec<ListEntry>) {
        // Clear existing children
        self.root.children.clear();

        // Group entries by parent directory
        let mut entries_by_parent: std::collections::HashMap<String, Vec<ListEntry>> =
            std::collections::HashMap::new();

        for entry in entries {
            let parent = self.get_parent_path(&entry.relative_path);
            entries_by_parent.entry(parent).or_default().push(entry);
        }

        // Build tree recursively
        let root_depth = self.root.depth;
        Self::build_subtree_static(&mut self.root, &entries_by_parent, ".", root_depth);
        self.cache_dirty = true;
    }

    fn build_subtree_static(
        node: &mut TreeNode,
        entries_by_parent: &std::collections::HashMap<String, Vec<ListEntry>>,
        current_path: &str,
        _parent_depth: usize,
    ) {
        if let Some(children_entries) = entries_by_parent.get(current_path) {
            for entry in children_entries {
                let child_depth = node.depth + 1;
                let mut child = TreeNode::new(entry.clone(), child_depth);

                if entry.is_directory {
                    // Recursively build children
                    Self::build_subtree_static(
                        &mut child,
                        entries_by_parent,
                        &entry.relative_path,
                        child_depth,
                    );
                }

                node.children.push(child);
            }

            // Sort children: directories first, then files, alphabetically
            node.children
                .sort_by(|a, b| match (a.entry.is_directory, b.entry.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.entry.name.cmp(&b.entry.name),
                });
        }
    }

    fn get_parent_path(&self, path: &str) -> String {
        if path == "." || !path.contains('/') {
            return ".".to_string();
        }

        let parts: Vec<&str> = path.rsplitn(2, '/').collect();
        if parts.len() == 2 {
            parts[1].to_string()
        } else {
            ".".to_string()
        }
    }

    /// Get flattened view of visible nodes
    pub fn flattened(&mut self) -> &[FlattenedNode] {
        if self.cache_dirty {
            self.flattened_cache.clear();
            self.root.flatten(&mut self.flattened_cache);
            self.cache_dirty = false;
        }
        &self.flattened_cache
    }

    /// Get current cursor position
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Get currently selected node
    pub fn selected_node(&mut self) -> Option<FlattenedNode> {
        let cursor = self.cursor;
        let flattened = self.flattened();
        flattened.get(cursor).cloned()
    }

    /// Move cursor up
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor down
    pub fn cursor_down(&mut self, max: usize) {
        if self.cursor + 1 < max {
            self.cursor += 1;
        }
    }

    /// Move cursor to top
    pub fn cursor_to_top(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to bottom
    pub fn cursor_to_bottom(&mut self, max: usize) {
        if max > 0 {
            self.cursor = max - 1;
        }
    }

    /// Set cursor to specific position
    pub fn set_cursor(&mut self, position: usize) {
        self.cursor = position;
    }

    /// Toggle expanded state of current node
    pub fn toggle_current_expansion(&mut self) -> bool {
        let flattened = self.flattened().to_vec();
        if let Some(node) = flattened.get(self.cursor) {
            // Allow expansion for any directory, even if it has no children yet
            // (children will be loaded on-demand)
            if node.entry.is_directory {
                // Find and toggle the actual node
                self.toggle_node_by_path(&node.entry.relative_path);
                self.cache_dirty = true;
                return true;
            }
        }
        false
    }

    /// Expand current node (right arrow) - only expands, doesn't toggle
    /// Returns (success, needs_children_load)
    pub fn expand_current(&mut self) -> (bool, bool) {
        let flattened = self.flattened().to_vec();
        if let Some(node) = flattened.get(self.cursor) {
            if node.entry.is_directory {
                let was_expanded = node.expanded;
                let has_children = node.has_children;

                if !was_expanded {
                    // Expand the node
                    self.expand_node_by_path(&node.entry.relative_path);
                    self.cache_dirty = true;
                    return (true, !has_children);
                }
            }
        }
        (false, false)
    }

    /// Collapse current node or move to parent (left arrow)
    /// Returns (collapsed_current, parent_index)
    pub fn collapse_current_or_goto_parent(&mut self) -> (bool, Option<usize>) {
        let flattened = self.flattened().to_vec();
        if let Some(node) = flattened.get(self.cursor) {
            if node.entry.is_directory && node.expanded {
                // Directory is expanded - collapse it
                self.collapse_node_by_path(&node.entry.relative_path);
                self.cache_dirty = true;
                return (true, None);
            } else {
                // File or collapsed directory - find parent
                let parent_depth = if node.depth > 0 { node.depth - 1 } else { 0 };

                // Look backwards to find the parent (first node with depth = parent_depth)
                for (i, ancestor) in flattened[..self.cursor].iter().enumerate().rev() {
                    if ancestor.depth == parent_depth && ancestor.entry.is_directory {
                        return (false, Some(i));
                    }
                }
            }
        }
        (false, None)
    }

    fn expand_node_by_path(&mut self, path: &str) -> bool {
        match self.root.find_mut(path) {
            Some(node) => {
                node.expanded = true;
                true
            }
            None => false,
        }
    }

    fn collapse_node_by_path(&mut self, path: &str) -> bool {
        match self.root.find_mut(path) {
            Some(node) => {
                node.expanded = false;
                true
            }
            None => false,
        }
    }

    fn toggle_node_by_path(&mut self, path: &str) -> bool {
        match self.root.find_mut(path) {
            Some(node) => {
                node.toggle_expanded();
                true
            }
            None => false,
        }
    }

    /// Update entry status
    pub fn update_entry_status(&mut self, path: &str, status: EntryStatus) {
        if self.root.update_entry_status(path, status) {
            self.cache_dirty = true;
        }
    }

    /// Get all entries (for compatibility)
    pub fn entries(&mut self) -> Vec<&ListEntry> {
        self.flattened().iter().map(|n| &n.entry).collect()
    }

    /// Add children to a specific directory node
    pub fn add_children_to_directory(&mut self, parent_path: &str, children: Vec<ListEntry>) {
        if let Some(node) = self.root.find_mut(parent_path) {
            let child_depth = node.depth + 1;
            for child_entry in children {
                node.children.push(TreeNode::new(child_entry, child_depth));
            }

            // Sort children: directories first, then files, alphabetically
            node.children
                .sort_by(|a, b| match (a.entry.is_directory, b.entry.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.entry.name.cmp(&b.entry.name),
                });

            self.cache_dirty = true;
        }
    }

    /// Check if a directory path has children loaded
    pub fn has_children_loaded(&self, path: &str) -> bool {
        self.root
            .find(path)
            .is_some_and(|node| !node.children.is_empty())
    }

    /// Expand all parent directories to make a path visible, then find its index in flattened view
    pub fn expand_and_find_path(&mut self, target_path: &str) -> Option<usize> {
        // Expand the path
        if self.root.expand_path_to(target_path) {
            self.cache_dirty = true;
            // Rebuild flattened cache
            let flattened = self.flattened();
            // Find the index
            flattened
                .iter()
                .position(|node| node.entry.relative_path == target_path)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(relative_path: &str, name: &str, is_directory: bool) -> ListEntry {
        ListEntry {
            relative_path: relative_path.to_string(),
            name: name.to_string(),
            selected: false,
            is_directory,
            status: if is_directory {
                EntryStatus::Directory
            } else {
                EntryStatus::Unknown
            },
        }
    }

    fn sample_entries() -> Vec<ListEntry> {
        vec![
            entry("dir_a", "dir_a", true),
            entry("dir_a/script1.sql", "script1.sql", false),
            entry("dir_a/script2.sql", "script2.sql", false),
            entry("dir_b", "dir_b", true),
            entry("dir_b/script3.sql", "script3.sql", false),
            entry("root.sql", "root.sql", false),
        ]
    }

    fn state_with_sample_tree() -> TreeState {
        let mut state = TreeState::new(PathBuf::from("base"));
        state.build_from_entries(sample_entries());
        state
    }

    #[test]
    fn find_mut_locates_nested_node_by_path() {
        let mut state = state_with_sample_tree();

        let node = state.root.find_mut("dir_a/script1.sql");

        assert!(node.is_some());
        assert_eq!(node.unwrap().entry.name, "script1.sql");
    }

    #[test]
    fn find_mut_returns_none_for_missing_path() {
        let mut state = state_with_sample_tree();

        assert!(state.root.find_mut("does/not/exist").is_none());
    }

    #[test]
    fn find_locates_root_by_dot_path() {
        let state = state_with_sample_tree();

        let node = state.root.find(".");

        assert!(node.is_some());
    }

    #[test]
    fn flatten_only_shows_top_level_when_collapsed() {
        let mut state = state_with_sample_tree();

        let flattened = state.flattened();

        // Root is always shown; dir_a and dir_b start collapsed, so their
        // children are not included, but root.sql is a direct child.
        let paths: Vec<&str> = flattened
            .iter()
            .map(|n| n.entry.relative_path.as_str())
            .collect();
        assert_eq!(paths, vec![".", "dir_a", "dir_b", "root.sql"]);
    }

    #[test]
    fn toggle_current_expansion_reveals_children() {
        let mut state = state_with_sample_tree();
        // Cursor starts at 0 (root); move to dir_a at index 1.
        state.set_cursor(1);

        let toggled = state.toggle_current_expansion();

        assert!(toggled);
        let paths: Vec<&str> = state
            .flattened()
            .iter()
            .map(|n| n.entry.relative_path.as_str())
            .collect();
        assert_eq!(
            paths,
            vec![
                ".",
                "dir_a",
                "dir_a/script1.sql",
                "dir_a/script2.sql",
                "dir_b",
                "root.sql"
            ]
        );
    }

    #[test]
    fn expand_current_only_expands_does_not_collapse() {
        let mut state = state_with_sample_tree();
        state.set_cursor(1);

        let (expanded_once, _) = state.expand_current();
        assert!(expanded_once);

        let (expanded_twice, _) = state.expand_current();
        assert!(!expanded_twice);
    }

    #[test]
    fn collapse_current_or_goto_parent_collapses_expanded_dir() {
        let mut state = state_with_sample_tree();
        state.set_cursor(1);
        state.toggle_current_expansion();

        let (collapsed, parent) = state.collapse_current_or_goto_parent();

        assert!(collapsed);
        assert_eq!(parent, None);
        let paths: Vec<&str> = state
            .flattened()
            .iter()
            .map(|n| n.entry.relative_path.as_str())
            .collect();
        assert_eq!(paths, vec![".", "dir_a", "dir_b", "root.sql"]);
    }

    #[test]
    fn collapse_current_or_goto_parent_navigates_to_parent_for_file() {
        let mut state = state_with_sample_tree();
        state.set_cursor(1);
        state.toggle_current_expansion(); // expand dir_a
                                          // Flattened: [".", "dir_a", "dir_a/script1.sql", "dir_a/script2.sql", "dir_b", "root.sql"]
        state.set_cursor(2); // dir_a/script1.sql

        let (collapsed, parent) = state.collapse_current_or_goto_parent();

        assert!(!collapsed);
        assert_eq!(parent, Some(1)); // index of dir_a
    }

    #[test]
    fn cursor_movement_respects_bounds() {
        let mut state = state_with_sample_tree();
        let max = state.flattened().len();

        state.cursor_up();
        assert_eq!(state.cursor(), 0);

        state.cursor_to_bottom(max);
        assert_eq!(state.cursor(), max - 1);

        state.cursor_down(max);
        assert_eq!(state.cursor(), max - 1);

        state.cursor_to_top();
        assert_eq!(state.cursor(), 0);
    }

    #[test]
    fn has_children_loaded_reflects_populated_children() {
        let state = state_with_sample_tree();

        assert!(state.has_children_loaded("dir_a"));
        assert!(!state.has_children_loaded("dir_b_that_does_not_exist"));
    }

    #[test]
    fn add_children_to_directory_appends_and_sorts_children() {
        let mut state = TreeState::new(PathBuf::from("base"));
        state.build_from_entries(vec![entry("dir_a", "dir_a", true)]);

        state.add_children_to_directory(
            "dir_a",
            vec![
                entry("dir_a/z.sql", "z.sql", false),
                entry("dir_a/a.sql", "a.sql", false),
            ],
        );

        assert!(state.has_children_loaded("dir_a"));
        state.expand_node_by_path("dir_a");
        let paths: Vec<&str> = state
            .flattened()
            .iter()
            .map(|n| n.entry.relative_path.as_str())
            .collect();
        assert_eq!(paths, vec![".", "dir_a", "dir_a/a.sql", "dir_a/z.sql"]);
    }

    #[test]
    fn expand_and_find_path_expands_ancestors_and_returns_index() {
        let mut state = state_with_sample_tree();

        let index = state.expand_and_find_path("dir_a/script2.sql");

        assert_eq!(
            index,
            Some(3),
            "expected dir_a/script2.sql to be found after expanding ancestors"
        );
        let paths: Vec<&str> = state
            .flattened()
            .iter()
            .map(|n| n.entry.relative_path.as_str())
            .collect();
        assert_eq!(
            paths,
            vec![
                ".",
                "dir_a",
                "dir_a/script1.sql",
                "dir_a/script2.sql",
                "dir_b",
                "root.sql"
            ]
        );
    }
}
