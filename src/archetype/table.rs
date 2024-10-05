use crate::core::{
    component::{Component, ComponentId},
    entity::Entity,
    internal::blob::{Blob, BlobCell},
};
use indexmap::{IndexMap, IndexSet};

pub struct ColumnCell {
    data: BlobCell,
}

impl ColumnCell {
    pub fn from<T: 'static>(value: T) -> Self {
        let data = BlobCell::new::<T>(value);

        Self { data }
    }

    pub fn value<T: 'static>(&self) -> &T {
        self.data.value()
    }

    pub fn value_mut<T: 'static>(&mut self) -> &mut T {
        self.data.value_mut()
    }

    pub fn into<T: 'static>(self) -> T {
        self.data.into()
    }
}

pub struct SelectedCell<'a> {
    column: &'a Column,
    index: usize,
}

impl<'a> SelectedCell<'a> {
    fn new(column: &'a Column, index: usize) -> Self {
        Self { column, index }
    }

    pub fn value<T: 'static>(&self) -> Option<&T> {
        self.column.get::<T>(self.index)
    }
}

pub struct SelectedCellMut<'a> {
    column: &'a mut Column,
    index: usize,
}

impl<'a> SelectedCellMut<'a> {
    fn new(column: &'a mut Column, index: usize) -> Self {
        Self { column, index }
    }

    pub fn value<T: 'static>(&self) -> Option<&T> {
        self.column.get::<T>(self.index)
    }

    pub fn value_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.column.get_mut::<T>(self.index)
    }
}

pub struct Column {
    data: Blob,
}

impl Column {
    pub fn new<T: 'static>() -> Self {
        Self {
            data: Blob::new::<T>(0),
        }
    }

    pub fn copy(column: &Column) -> Self {
        Column {
            data: Blob::with_layout(column.data.layout().clone(), 0, column.data.drop().copied()),
        }
    }

    pub fn copy_cell(cell: &ColumnCell) -> Self {
        Column {
            data: Blob::with_layout(cell.data.layout().clone(), 0, cell.data.drop().copied()),
        }
    }

    pub fn get<T: 'static>(&self, index: usize) -> Option<&T> {
        self.data.get::<T>(index)
    }

    pub fn get_mut<T: 'static>(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut::<T>(index)
    }

    pub fn push<T: 'static>(&mut self, value: T) {
        self.data.push(value)
    }

    pub fn insert<T: 'static>(&mut self, index: usize, value: T) {
        self.data.insert(index, value)
    }

    pub fn extend(&mut self, column: Column) {
        self.data.extend(column.data)
    }

    pub fn remove<T: 'static>(&mut self, index: usize) -> T {
        self.data.remove(index)
    }

    pub fn swap_remove<T: 'static>(&mut self, index: usize) -> T {
        self.data.swap_remove(index)
    }

    pub fn select(&self, index: usize) -> Option<SelectedCell> {
        if index >= self.len() {
            None
        } else {
            Some(SelectedCell::new(self, index))
        }
    }

    pub fn select_mut(&mut self, index: usize) -> Option<SelectedCellMut> {
        if index >= self.len() {
            None
        } else {
            Some(SelectedCellMut::new(self, index))
        }
    }

    pub fn push_cell(&mut self, cell: ColumnCell) {
        self.data.extend(cell.data.into())
    }

    pub fn insert_cell(&mut self, index: usize, cell: ColumnCell) {
        self.data.insert_blob(index, cell.data.into())
    }

    pub fn remove_cell(&mut self, index: usize) -> ColumnCell {
        let data = self.data.remove_blob(index).into();
        ColumnCell { data }
    }

    pub fn swap_remove_cell(&mut self, index: usize) -> ColumnCell {
        let data = self.data.swap_remove_blob(index).into();
        ColumnCell { data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.len() == 0
    }

    pub fn clear(&mut self) {
        self.data.clear()
    }
}

impl From<ColumnCell> for Column {
    fn from(cell: ColumnCell) -> Self {
        Column {
            data: Blob::from(cell.data),
        }
    }
}

pub struct Row {
    components: IndexMap<ComponentId, ColumnCell>,
}

impl Row {
    pub fn new() -> Self {
        Row {
            components: IndexMap::new(),
        }
    }

    pub fn ids(&self) -> impl Iterator<Item = &ComponentId> {
        self.components.keys()
    }

    pub fn get<C: Component>(&self) -> Option<&C> {
        self.components
            .get(&ComponentId::of::<C>())
            .and_then(|cell| Some(cell.value::<C>()))
    }

    pub fn get_mut<C: Component>(&mut self) -> Option<&mut C> {
        self.components
            .get_mut(&ComponentId::of::<C>())
            .and_then(|cell| Some(cell.value_mut::<C>()))
    }

    pub fn add_component<C: Component>(&mut self, component: C) -> Option<ColumnCell> {
        let id = ComponentId::of::<C>();
        self.components.insert(id, ColumnCell::from(component))
    }

    pub fn replace_component<C: Component>(&mut self, component: C) -> Option<(ComponentId, C)> {
        let id = ComponentId::of::<C>();
        let prev = self.components.insert(id, ColumnCell::from(component))?;
        let prev = prev.into();

        Some((id, prev))
    }

    pub fn remove_component<C: Component>(&mut self) -> Option<(ComponentId, C)> {
        let id = ComponentId::of::<C>();
        let prev = self
            .components
            .shift_remove(&id)
            .and_then(|cell| Some(cell.into()))?;

        Some((id, prev))
    }

    pub fn add_cell(&mut self, id: ComponentId, cell: ColumnCell) -> Option<ColumnCell> {
        self.components.insert(id, cell)
    }

    pub fn remove_cell(&mut self, id: &ComponentId) -> Option<ColumnCell> {
        self.components.shift_remove(id)
    }

    pub fn contains<C: Component>(&self) -> bool {
        self.components.contains_key(&ComponentId::of::<C>())
    }

    pub fn contains_id(&self, id: &ComponentId) -> bool {
        self.components.contains_key(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ComponentId, &ColumnCell)> {
        self.components.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ComponentId, &mut ColumnCell)> {
        self.components.iter_mut()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = (ComponentId, ColumnCell)> + '_ {
        self.components.drain(..)
    }

    pub fn sort(&mut self) {
        self.components.sort_by(|a, _, b, _| a.cmp(&b));
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    pub fn clear(&mut self) {
        self.components.clear();
    }

    pub fn into_table(mut self, entity: Entity) -> Table {
        let mut builder = TableBuilder::new();
        for (id, cell) in self.components.drain(..) {
            builder.add_column(id, Column::from(cell));
        }
        let mut table = builder.build();
        table.rows.insert(entity);

        table
    }

    pub fn table_builder(&self) -> TableBuilder {
        let mut builder = TableBuilder::new();
        for (id, cell) in self.components.iter() {
            builder.add_column(*id, Column::copy_cell(cell));
        }

        builder
    }
}

impl IntoIterator for Row {
    type Item = (ComponentId, ColumnCell);
    type IntoIter = indexmap::map::IntoIter<ComponentId, ColumnCell>;

    fn into_iter(self) -> Self::IntoIter {
        self.components.into_iter()
    }
}

impl<'a> IntoIterator for &'a Row {
    type Item = (&'a ComponentId, &'a ColumnCell);
    type IntoIter = indexmap::map::Iter<'a, ComponentId, ColumnCell>;

    fn into_iter(self) -> Self::IntoIter {
        self.components.iter()
    }
}

pub struct SelectedRow<'a> {
    index: usize,
    row: IndexMap<ComponentId, &'a Column>,
}

impl<'a> SelectedRow<'a> {
    pub fn new(index: usize, row: IndexMap<ComponentId, &'a Column>) -> Self {
        Self { index, row }
    }

    pub fn get<C: Component>(&self) -> Option<&C> {
        self.row
            .get(&ComponentId::of::<C>())
            .and_then(|column| column.get(self.index))
    }

    pub fn contains<C: Component>(&self) -> bool {
        self.row.contains_key(&ComponentId::of::<C>())
    }

    pub fn contains_id(&self, id: &ComponentId) -> bool {
        self.row.contains_key(id)
    }
}

pub struct SelectedRowMut<'a> {
    index: usize,
    row: IndexMap<ComponentId, &'a mut Column>,
}

impl<'a> SelectedRowMut<'a> {
    pub fn new(index: usize, row: IndexMap<ComponentId, &'a mut Column>) -> Self {
        Self { index, row }
    }

    pub fn get<C: Component>(&self) -> Option<&C> {
        self.row
            .get(&ComponentId::of::<C>())
            .and_then(|column| column.get(self.index))
    }

    pub fn get_mut<C: Component>(&mut self) -> Option<&mut C> {
        self.row
            .get_mut(&ComponentId::of::<C>())
            .and_then(|column| column.get_mut(self.index))
    }

    pub fn contains<C: Component>(&self) -> bool {
        self.row.contains_key(&ComponentId::of::<C>())
    }

    pub fn contains_id(&self, id: &ComponentId) -> bool {
        self.row.contains_key(id)
    }
}

pub struct TableBuilder {
    components: IndexMap<ComponentId, Column>,
}

impl TableBuilder {
    pub fn new() -> Self {
        TableBuilder {
            components: IndexMap::new(),
        }
    }

    pub fn components(&self) -> impl Iterator<Item = &ComponentId> {
        self.components.keys()
    }

    pub fn add_component<C: Component>(&mut self) {
        self.components
            .insert(ComponentId::of::<C>(), Column::new::<C>());
    }

    pub fn remove_component<C: Component>(&mut self) {
        self.components.shift_remove(&ComponentId::of::<C>());
    }

    pub fn add_column(&mut self, id: ComponentId, column: Column) {
        self.components.insert(id, column);
    }

    pub fn remove_column(&mut self, id: &ComponentId) {
        self.components.shift_remove(id);
    }

    pub fn build(self) -> Table {
        Table {
            rows: IndexSet::new(),
            components: self.components,
        }
    }
}

pub struct Table {
    rows: IndexSet<Entity>,
    components: IndexMap<ComponentId, Column>,
}

impl Table {
    pub fn builder() -> TableBuilder {
        TableBuilder::new()
    }

    pub fn entities(&self) -> &IndexSet<Entity> {
        &self.rows
    }

    pub fn components(&self) -> impl Iterator<Item = &ComponentId> {
        self.components.keys()
    }

    pub fn contains(&self, entity: &Entity) -> bool {
        self.rows.contains(entity)
    }

    pub fn has_component(&self, id: &ComponentId) -> bool {
        self.components.contains_key(id)
    }

    pub fn get_component<C: Component>(&self, entity: &Entity) -> Option<&C> {
        let column = self.components.get(&ComponentId::of::<C>())?;
        let index = self.rows.get_index_of(entity)?;
        column.get(index)
    }

    pub fn get_component_mut<C: Component>(&mut self, entity: &Entity) -> Option<&mut C> {
        let column = self.components.get_mut(&ComponentId::of::<C>())?;
        let index = self.rows.get_index_of(entity)?;
        column.get_mut(index)
    }

    pub fn add_entity(&mut self, entity: Entity, mut row: Row) {
        self.rows.insert(entity);
        for (id, cell) in row.drain() {
            let column = match self.components.get_mut(&id) {
                Some(column) => column,
                None => continue,
            };

            column.push_cell(cell);
        }
    }

    pub fn remove_entity(&mut self, entity: &Entity) -> Option<Row> {
        let index = self.rows.get_index_of(entity)?;
        self.rows.swap_remove_index(index);
        let mut row = Row::new();
        for (id, column) in self.components.iter_mut() {
            let cell = column.remove_cell(index);
            row.add_cell(*id, cell);
        }

        Some(row)
    }
}
