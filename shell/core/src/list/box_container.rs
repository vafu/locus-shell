use std::ptr::NonNull;

use gtk::glib::Quark;
use gtk::prelude::{BoxExt, Cast, IsA, ObjectExt};
use relm4::component::ComponentController;
use relm4::{Component, Controller};

use super::{ComponentListBoxExt, ComponentListUpdate};

impl<T> ComponentListBoxExt for T
where
    T: IsA<gtk::Box>,
{
    fn set_component_list<C>(&self, update: ComponentListUpdate<'_, C>)
    where
        C: Component,
        C::Init: Clone + PartialEq + 'static,
        C::Root: AsRef<gtk::Widget> + Clone + std::fmt::Debug,
    {
        let container = self.upcast_ref::<gtk::Box>();
        let key = component_list_key::<C>();
        let host = component_list_host::<C>(container, key);
        host.reconcile(container, update.items);
    }
}

struct ComponentListHost<C>
where
    C: Component,
{
    rows: Vec<ComponentListRow<C>>,
}

impl<C> Default for ComponentListHost<C>
where
    C: Component,
{
    fn default() -> Self {
        Self { rows: Vec::new() }
    }
}

impl<C> ComponentListHost<C>
where
    C: Component,
    C::Init: Clone + PartialEq + 'static,
    C::Root: AsRef<gtk::Widget> + Clone + std::fmt::Debug,
{
    fn reconcile(&mut self, container: &gtk::Box, items: &[C::Init]) {
        for row in &self.rows {
            container.remove(row.widget());
        }

        let mut old_rows = std::mem::take(&mut self.rows);
        let mut rows = Vec::with_capacity(items.len());

        for item in items {
            let row = old_rows
                .iter()
                .position(|row| &row.item == item)
                .map(|index| old_rows.remove(index))
                .unwrap_or_else(|| ComponentListRow::new(item.clone()));
            container.append(row.widget());
            rows.push(row);
        }

        self.rows = rows;
    }
}

struct ComponentListRow<C>
where
    C: Component,
{
    item: C::Init,
    controller: Controller<C>,
}

impl<C> ComponentListRow<C>
where
    C: Component,
    C::Init: Clone,
{
    fn new(item: C::Init) -> Self {
        let controller = C::builder().launch(item.clone()).detach();
        Self { item, controller }
    }

    fn widget(&self) -> &gtk::Widget
    where
        C::Root: AsRef<gtk::Widget>,
    {
        self.controller.widget().as_ref()
    }
}

fn component_list_key<C>() -> Quark
where
    C: Component,
{
    Quark::from_str(std::any::type_name::<ComponentListHost<C>>())
}

fn component_list_host<C>(container: &gtk::Box, key: Quark) -> &mut ComponentListHost<C>
where
    C: Component,
{
    // GTK object data owns the row controllers for this container. The quark is
    // derived from the row component type, so the downcast type matches writes.
    unsafe {
        if container.qdata::<ComponentListHost<C>>(key).is_none() {
            container.set_qdata(key, ComponentListHost::<C>::default());
        }

        let host: NonNull<ComponentListHost<C>> = container
            .qdata(key)
            .expect("component list host was just installed");
        host.as_ptr()
            .as_mut()
            .expect("component list host pointer must be valid")
    }
}
