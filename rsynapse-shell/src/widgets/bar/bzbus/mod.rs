mod source;
#[cfg(test)]
mod test;
mod view;

pub(in crate::widgets::bar) use source::bzbus_status;
pub(crate) use view::BzBusView;
