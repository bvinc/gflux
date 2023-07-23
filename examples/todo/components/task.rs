use super::list::ListComponent;
use crate::{AppState, Task};
use gflux::{Component, ComponentCtx};
use glib::clone;
use gtk::{prelude::*, Align};

pub struct TaskComponent {
    hbox: gtk::Box,
    label: gtk::Label,
}

impl Component for TaskComponent {
    type GlobalModel = AppState;
    type Model = Task;
    type Widget = gtk::Box;
    type Params = ComponentCtx<ListComponent>;

    fn widget(&self) -> Self::Widget {
        self.hbox.clone()
    }

    fn build(ctx: ComponentCtx<Self>, list_ctx: ComponentCtx<ListComponent>) -> Self {
        let checkbox = gtk::CheckButton::new();
        checkbox.connect_toggled(clone!(@strong ctx => move |cb| {
            ctx.with_model_mut(|task| task.done = cb.is_active());
        }));

        let task_id = ctx.with_model(|task| task.id);

        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let label = gtk::Label::new(None);
        label.set_hexpand(true);
        label.set_halign(Align::Start);
        let del_button = gtk::Button::from_icon_name("edit-delete");
        del_button.connect_clicked(move |_| {
            list_ctx.with_model_mut(move |tasks| tasks.remove_task(task_id));
        });
        hbox.append(&checkbox);
        hbox.append(&label);
        hbox.append(&del_button);

        // rebuild will be called immediately afterwards
        Self { hbox, label }
    }

    fn rebuild(&mut self, ctx: ComponentCtx<Self>) {
        let name = ctx.with_model(|task| task.name.clone());
        if ctx.with_model(|task| task.done) {
            let markup = format!("<s>{}</s>", glib::markup_escape_text(&name));
            self.label.set_markup(&markup);
        } else {
            self.label.set_text(&name);
        }
    }
}
