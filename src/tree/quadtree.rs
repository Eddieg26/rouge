use crate::primitives::{intersect::Intersect2D, object::Object2D, rect::Rect};

pub struct TreeNode<T: Object2D> {
    rect: Rect,
    objects: Vec<T>,
    children: Option<[Box<TreeNode<T>>; 4]>,
    depth: usize,
    max_depth: usize,
    max_objects: usize,
}

impl<T: Object2D> TreeNode<T> {
    pub fn insert(&mut self, object: T) {
        if self.depth == self.max_depth {
            self.objects.push(object);
            return;
        }

        if self.objects.len() == self.max_objects && self.children.is_none() {
            self.split();
        }

        let children = self.children.as_mut().unwrap();

        for child in children.iter_mut() {
            if child.rect.intersects_rect(&object.bounds()) {
                child.insert(object);
                return;
            }
        }

        self.objects.push(object);
    }

    pub fn query(&self, rect: &Rect) -> Vec<&T> {
        let mut objects = Vec::new();

        for object in self.objects.iter() {
            if rect.intersects_rect(&object.bounds()) {
                objects.push(object);
            }
        }

        if self.children.is_some() {
            let children = self.children.as_ref().unwrap();

            for child in children.iter() {
                if child.rect.intersects_rect(rect) {
                    objects.append(&mut child.query(rect));
                }
            }
        }

        objects
    }

    pub fn query_mut(&mut self, rect: &Rect) -> Vec<&mut T> {
        let mut objects = Vec::new();

        if self.depth == self.max_depth {
            for object in self.objects.iter_mut() {
                if rect.intersects_rect(&object.bounds()) {
                    objects.push(object);
                }
            }
        } else {
            if self.children.is_some() {
                let children = self.children.as_mut().unwrap();

                for child in children.iter_mut() {
                    if child.rect.intersects_rect(rect) {
                        objects.append(&mut child.query_mut(rect));
                    }
                }
            }
        }

        objects
    }

    pub fn clear(&mut self) {
        self.objects.clear();

        if self.children.is_some() {
            let children = self.children.as_mut().unwrap();

            for child in children.iter_mut() {
                child.clear();
            }
        }
    }

    fn split(&mut self) {
        let half_size = self.rect.size() / 2.0;

        let children = [
            Box::new(TreeNode {
                rect: Rect::new(self.rect.top(), self.rect.left(), half_size.x, half_size.y),
                objects: Vec::new(),
                children: None,
                depth: self.depth + 1,
                max_depth: self.max_depth,
                max_objects: self.max_objects,
            }),
            Box::new(TreeNode {
                rect: Rect::new(
                    self.rect.top(),
                    self.rect.left() + half_size.y,
                    half_size.x,
                    half_size.y,
                ),
                objects: Vec::new(),
                children: None,
                depth: self.depth + 1,
                max_depth: self.max_depth,
                max_objects: self.max_objects,
            }),
            Box::new(TreeNode {
                rect: Rect::new(
                    self.rect.top() + half_size.x,
                    self.rect.left() + half_size.y,
                    half_size.x,
                    half_size.y,
                ),
                objects: Vec::new(),
                children: None,
                depth: self.depth + 1,
                max_depth: self.max_depth,
                max_objects: self.max_objects,
            }),
            Box::new(TreeNode {
                rect: Rect::new(
                    self.rect.top(),
                    self.rect.left() + half_size.y,
                    half_size.x,
                    half_size.y,
                ),
                objects: Vec::new(),
                children: None,
                depth: self.depth + 1,
                max_depth: self.max_depth,
                max_objects: self.max_objects,
            }),
        ];

        let mut objects = Vec::new();
        std::mem::swap(&mut objects, &mut self.objects);

        self.children = Some(children);

        for object in objects {
            self.insert(object);
        }
    }

    pub fn remove(&mut self, id: T::Identifier) -> Option<T> {
        if let Some(index) = self.objects.iter().position(|o| o.id() == id) {
            return Some(self.objects.remove(index));
        } else if self.children.is_some() {
            let children = self.children.as_mut().unwrap();

            for child in children.iter_mut() {
                if let Some(object) = child.remove(id) {
                    return Some(object);
                }
            }
        }

        None
    }

    pub fn objects(&self) -> &Vec<T> {
        &self.objects
    }

    pub fn children(&self) -> Vec<&mut Box<TreeNode<T>>> {
        if let Some(children) = &self.children {
            let mut child_list = vec![];
            for child in children.iter() {
                child_list.append(&mut child.children());
            }

            child_list
        } else {
            Vec::new()
        }
    }

    pub fn children_mut(&mut self) -> Option<&mut [Box<TreeNode<T>>; 4]> {
        self.children.as_mut()
    }

    pub fn objects_mut(&mut self) -> &mut Vec<T> {
        &mut self.objects
    }

    pub fn pop(&mut self) -> Option<T> {
        self.objects.pop()
    }

    pub fn ids(&self) -> Vec<T::Identifier> {
        let mut ids = Vec::new();

        for object in self.objects.iter() {
            ids.push(object.id());
        }

        if self.children.is_some() {
            let children = self.children.as_ref().unwrap();

            for child in children.iter() {
                ids.append(&mut child.ids());
            }
        }

        ids
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn max_depth(&self) -> usize {
        self.max_depth
    }

    pub fn max_objects(&self) -> usize {
        self.max_objects
    }
}

pub struct QuadTree<T: Object2D> {
    root: Box<TreeNode<T>>,
}

impl<T: Object2D> QuadTree<T> {
    pub fn new(rect: Rect, max_depth: usize, max_objects: usize) -> QuadTree<T> {
        QuadTree {
            root: Box::new(TreeNode {
                rect,
                objects: Vec::new(),
                children: None,
                depth: 0,
                max_depth,
                max_objects,
            }),
        }
    }

    pub fn root(&self) -> &Box<TreeNode<T>> {
        &self.root
    }

    pub fn root_mut(&mut self) -> &mut Box<TreeNode<T>> {
        &mut self.root
    }

    pub fn insert(&mut self, object: T) {
        self.root.insert(object);
    }

    pub fn query(&self, rect: &Rect) -> Vec<&T> {
        self.root.query(rect)
    }

    pub fn query_mut(&mut self, rect: &Rect) -> Vec<&mut T> {
        self.root.query_mut(rect)
    }

    pub fn clear(&mut self) {
        self.root.clear();
    }
}
