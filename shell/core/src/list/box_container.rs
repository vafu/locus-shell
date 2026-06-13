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
        let mut index = 0;
        while index < self.rows.len() {
            if items.iter().any(|item| item == &self.rows[index].item) {
                index += 1;
            } else {
                let row = self.rows.remove(index);
                container.remove(row.widget());
            }
        }

        for (target_index, item) in items.iter().enumerate() {
            let current_index = self.rows.iter().position(|row| &row.item == item);
            match current_index {
                Some(current_index) if current_index != target_index => {
                    let row = self.rows.remove(current_index);
                    self.rows.insert(target_index, row);
                    move_widget(container, &self.rows, target_index);
                }
                Some(_) => {}
                None => {
                    let row = ComponentListRow::new(item.clone());
                    insert_widget(container, &self.rows, target_index, row.widget());
                    self.rows.insert(target_index, row);
                }
            }
        }
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

fn insert_widget<C>(
    container: &gtk::Box,
    rows: &[ComponentListRow<C>],
    target_index: usize,
    widget: &gtk::Widget,
) where
    C: Component,
    C::Init: Clone,
    C::Root: AsRef<gtk::Widget>,
{
    if target_index == 0 {
        container.prepend(widget);
        return;
    }

    let previous = rows[target_index - 1].widget();
    container.insert_child_after(widget, Some(previous));
}

fn move_widget<C>(container: &gtk::Box, rows: &[ComponentListRow<C>], target_index: usize)
where
    C: Component,
    C::Init: Clone,
    C::Root: AsRef<gtk::Widget>,
{
    let widget = rows[target_index].widget();
    if target_index == 0 {
        container.reorder_child_after(widget, None::<&gtk::Widget>);
        return;
    }

    let previous = rows[target_index - 1].widget();
    container.reorder_child_after(widget, Some(previous));
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
