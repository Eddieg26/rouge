use crate::primitives::{bounds::Bounds, intersect::Intersect3D, object::Object3D};

pub struct TreeNode<T: Object3D> {
    bounds: Bounds,
    objects: Vec<T>,
    children: Option<[Box<TreeNode<T>>; 8]>,
    depth: usize,
    max_depth: usize,
    max_objects: usize,
}

impl<T: Object3D> TreeNode<T> {
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
            if child.bounds.intersects_bounds(&object.bounds()) {
                child.insert(object);
                return;
            }
        }

        self.objects.push(object);
    }

    pub fn query(&self, bounds: &Bounds) -> Vec<&T> {
        let mut objects = Vec::new();

        for object in self.objects.iter() {
            if bounds.intersects_bounds(&object.bounds()) {
                objects.push(object);
            }
        }

        if self.children.is_some() {
            let children = self.children.as_ref().unwrap();

            for child in children.iter() {
                if child.bounds.intersects_bounds(bounds) {
                    objects.append(&mut child.query(bounds));
                }
            }
        }

        objects
    }

    pub fn query_mut(&mut self, bounds: &Bounds) -> Vec<&mut T> {
        let mut objects = Vec::new();

        if self.depth == self.max_depth {
            for object in self.objects.iter_mut() {
                if bounds.intersects_bounds(&object.bounds()) {
                    objects.push(object);
                }
            }
        } else {
            if self.children.is_some() {
                let children = self.children.as_mut().unwrap();

                for child in children.iter_mut() {
                    if child.bounds.intersects_bounds(bounds) {
                        objects.append(&mut child.query_mut(bounds));
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

    pub fn split(&mut self) {
        let half_size = self.bounds.size / 2.0;

        let children = [
            Box::new(TreeNode {
                bounds: Bounds::new(self.bounds.center + half_size * glam::Vec3::ONE, half_size),
                objects: Vec::new(),
                children: None,
                depth: self.depth + 1,
                max_depth: self.max_depth,
                max_objects: self.max_objects,
            }),
            Box::new(TreeNode {
                bounds: Bounds::new(
                    self.bounds.center + half_size * glam::Vec3::new(-1.0, 1.0, 1.0),
                    half_size,
                ),
                objects: Vec::new(),
                children: None,
                depth: self.depth + 1,
                max_depth: self.max_depth,
                max_objects: self.max_objects,
            }),
            Box::new(TreeNode {
                bounds: Bounds::new(
                    self.bounds.center + half_size * glam::Vec3::new(1.0, -1.0, 1.0),
                    half_size,
                ),
                objects: Vec::new(),
                children: None,
                depth: self.depth + 1,
                max_depth: self.max_depth,
                max_objects: self.max_objects,
            }),
            Box::new(TreeNode {
                bounds: Bounds::new(
                    self.bounds.center + half_size * glam::Vec3::new(-1.0, -1.0, 1.0),
                    half_size,
                ),
                objects: Vec::new(),
                children: None,
                depth: self.depth + 1,
                max_depth: self.max_depth,
                max_objects: self.max_objects,
            }),
            Box::new(TreeNode {
                bounds: Bounds::new(
                    self.bounds.center + half_size * glam::Vec3::new(1.0, 1.0, -1.0),
                    half_size,
                ),
                objects: Vec::new(),
                children: None,
                depth: self.depth + 1,
                max_depth: self.max_depth,
                max_objects: self.max_objects,
            }),
            Box::new(TreeNode {
                bounds: Bounds::new(
                    self.bounds.center + half_size * glam::Vec3::new(-1.0, 1.0, -1.0),
                    half_size,
                ),
                objects: Vec::new(),
                children: None,
                depth: self.depth + 1,
                max_depth: self.max_depth,
                max_objects: self.max_objects,
            }),
            Box::new(TreeNode {
                bounds: Bounds::new(
                    self.bounds.center + half_size * glam::Vec3::new(1.0, -1.0, -1.0),
                    half_size,
                ),
                objects: Vec::new(),
                children: None,
                depth: self.depth + 1,
                max_depth: self.max_depth,
                max_objects: self.max_objects,
            }),
            Box::new(TreeNode {
                bounds: Bounds::new(
                    self.bounds.center + half_size * glam::Vec3::ONE * -1.0,
                    half_size,
                ),
                objects: Vec::new(),
                children: None,
                depth: self.depth + 1,
                max_depth: self.max_depth,
                max_objects: self.max_objects,
            }),
        ];

        self.children = Some(children);

        let mut objects = Vec::new();
        std::mem::swap(&mut objects, &mut self.objects);

        for object in objects.into_iter() {
            self.insert(object);
        }
    }
}

pub struct OctTree<T: Object3D> {
    root: TreeNode<T>,
}

impl<T: Object3D> OctTree<T> {
    pub fn new(bounds: Bounds, max_depth: usize, max_objects: usize) -> Self {
        Self {
            root: TreeNode {
                bounds,
                objects: Vec::new(),
                children: None,
                depth: 0,
                max_depth,
                max_objects,
            },
        }
    }

    pub fn root(&self) -> &TreeNode<T> {
        &self.root
    }

    pub fn root_mut(&mut self) -> &mut TreeNode<T> {
        &mut self.root
    }

    pub fn insert(&mut self, object: T) {
        self.root.insert(object);
    }

    pub fn query(&self, bounds: &Bounds) -> Vec<&T> {
        self.root.query(bounds)
    }

    pub fn query_mut(&mut self, bounds: &Bounds) -> Vec<&mut T> {
        self.root.query_mut(bounds)
    }

    pub fn clear(&mut self) {
        self.root.clear();
    }
}
