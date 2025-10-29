use std::path::PathBuf;
use crate::entries::{EntryStatus, ListEntry};

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

    /// Count total files in this node and all children
    pub fn count_files(&self) -> usize {
        let mut count = if !self.entry.is_directory { 1 } else { 0 };
        for child in &self.children {
            count += child.count_files();
        }
        count
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

    /// Recursively find and update an entry by path
    pub fn update_entry_status(&mut self, path: &str, status: EntryStatus) -> bool {
        if self.entry.relative_path == path {
            self.entry.status = status.clone();
            return true;
        }

        for child in &mut self.children {
            if child.update_entry_status(path, status.clone()) {
                return true;
            }
        }

        false
    }

    /// Recursively find and update selection by path
    pub fn update_selection(&mut self, path: &str, selected: bool) -> bool {
        if self.entry.relative_path == path {
            self.entry.selected = selected;
            return true;
        }

        for child in &mut self.children {
            if child.update_selection(path, selected) {
                return true;
            }
        }

        false
    }

    /// Toggle expanded state
    pub fn toggle_expanded(&mut self) {
        if self.entry.is_directory {
            self.expanded = !self.expanded;
        }
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

        if self.expanded || !self.entry.is_directory {
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
            entries_by_parent
                .entry(parent)
                .or_insert_with(Vec::new)
                .push(entry);
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
                    Self::build_subtree_static(&mut child, entries_by_parent, &entry.relative_path, child_depth);
                }

                node.children.push(child);
            }

            // Sort children: directories first, then files, alphabetically
            node.children.sort_by(|a, b| {
                match (a.entry.is_directory, b.entry.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.entry.name.cmp(&b.entry.name),
                }
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

    /// Toggle expanded state of current node
    pub fn toggle_current_expansion(&mut self) -> bool {
        let flattened = self.flattened().to_vec();
        if let Some(node) = flattened.get(self.cursor) {
            if node.has_children {
                // Find and toggle the actual node
                self.toggle_node_by_path(&node.entry.relative_path);
                self.cache_dirty = true;
                return true;
            }
        }
        false
    }

    fn toggle_node_by_path(&mut self, path: &str) -> bool {
        Self::toggle_node_recursive_static(&mut self.root, path)
    }

    fn toggle_node_recursive_static(node: &mut TreeNode, path: &str) -> bool {
        if node.entry.relative_path == path {
            node.toggle_expanded();
            return true;
        }

        for child in &mut node.children {
            if Self::toggle_node_recursive_static(child, path) {
                return true;
            }
        }

        false
    }

    /// Update entry status
    pub fn update_entry_status(&mut self, path: &str, status: EntryStatus) {
        if self.root.update_entry_status(path, status) {
            self.cache_dirty = true;
        }
    }

    /// Update selection
    pub fn update_selection(&mut self, path: &str, selected: bool) {
        if self.root.update_selection(path, selected) {
            self.cache_dirty = true;
        }
    }

    /// Get all entries (for compatibility)
    pub fn entries(&mut self) -> Vec<&ListEntry> {
        self.flattened().iter().map(|n| &n.entry).collect()
    }
}
