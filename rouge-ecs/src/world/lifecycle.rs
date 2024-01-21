use crate::{
    archetype::{ArchetypeId, Archetypes},
    core::{Component, ComponentId, Entity},
    storage::{
        blob::Blob,
        sparse::SparseSet,
        table::{Column, Table, TableId, TableRow, Tables},
    },
};

pub struct Lifecycle;

impl Lifecycle {
    pub fn create_entity(entity: Entity, archetypes: &mut Archetypes, tables: &mut Tables<Entity>) {
        let table_id = ArchetypeId::new(&[]).into();
        let table = if let Some(table) = tables.get_mut(table_id) {
            table
        } else {
            let table = Table::<Entity>::with_capacity(1).build();
            tables.insert(table);
            tables.get_mut(table_id).unwrap()
        };

        archetypes.add_entity(entity);
        table.add_row(entity, TableRow::new(entity, SparseSet::new()));
    }

    pub fn add_component<C: Component>(
        entity: Entity,
        component_id: ComponentId,
        component: C,
        archetypes: &mut Archetypes,
        tables: &mut Tables<Entity>,
    ) {
        let mut blob = Blob::new::<C>();
        blob.push(component);

        let archetype = archetypes.archetype_id(entity).cloned().unwrap();
        let new_archetype_id = archetypes.add_component(entity, component_id).unwrap();

        let old_table_id: TableId = archetype.into();

        let mut row = tables
            .get_mut(old_table_id)
            .unwrap()
            .remove_row(entity)
            .unwrap();

        row.insert(component_id.into(), Column::from_blob(blob));

        let new_table_id: TableId = new_archetype_id.into();
        let new_table = if let Some(table) = tables.get_mut(new_table_id) {
            table
        } else {
            let table = Table::<Entity>::from_row(&row, 1);
            tables.insert(table);
            tables.get_mut(new_table_id).unwrap()
        };

        new_table.add_row(entity, row);
    }

    pub fn remove_component(
        entity: Entity,
        component_id: ComponentId,
        archetypes: &mut Archetypes,
        tables: &mut Tables<Entity>,
    ) {
        if !archetypes.has(entity, component_id) {
            return;
        }

        let archetype = archetypes.archetype_id(entity).cloned().unwrap();
        let new_archetype_id = archetypes.remove_component(entity, component_id).unwrap();

        let old_table_id: TableId = archetype.into();

        let mut row = tables
            .get_mut(old_table_id)
            .unwrap()
            .remove_row(entity)
            .unwrap();

        row.remove(component_id.into());

        let new_table_id: TableId = new_archetype_id.into();
        let new_table = if let Some(table) = tables.get_mut(new_table_id) {
            table
        } else {
            let table = Table::<Entity>::from_row(&row, 1);
            tables.insert(table);
            tables.get_mut(new_table_id).unwrap()
        };

        new_table.add_row(entity, row);
    }

    pub fn delete_entity(
        entity: Entity,
        archetypes: &mut Archetypes,
        tables: &mut Tables<Entity>,
    ) -> Option<TableRow<Entity>> {
        let archetype = archetypes.delete_entity(entity)?;
        let table_id = (*archetype).into();

        let table = tables.get_mut(table_id)?;
        table.remove_row(entity)
    }
}
