use super::task::TaskComponent;
use crate::{AppState, Tasks};
use gflux::{Component, ComponentCtx, ComponentHandle};
use gtk::prelude::*;
use std::collections::{BTreeMap, HashSet};

pub struct ListComponent {
    vbox: gtk::Box,
    summary: gtk::Label,
    task_comps: BTreeMap<u64, ComponentHandle<TaskComponent>>,
}

impl Component for ListComponent {
    type GlobalModel = AppState;
    type Model = Tasks;
    type Widget = gtk::Box;
    type Params = ();

    fn widget(&self) -> Self::Widget {
        self.vbox.clone()
    }

    fn build(ctx: ComponentCtx<Self>, _params: ()) -> Self {
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let entry = gtk::Entry::new();
        entry.set_placeholder_text(Some("Add a task"));
        entry.connect_activate(move |entry| {
            let text = entry.text();
            ctx.with_model_mut(move |tasks| tasks.add_task(text.as_str()));
            entry.set_text("");
        });

        let summary = gtk::Label::new(None);
        summary.set_halign(gtk::Align::Start);

        vbox.append(&entry);
        vbox.append(&summary);

        let task_comps = BTreeMap::new();

        // rebuild will be called immediately afterwards
        Self {
            vbox,
            summary,
            task_comps,
        }
    }

    fn rebuild(&mut self, ctx: ComponentCtx<Self>) {
        let task_ids: HashSet<u64> = ctx.with_model(|task| task.map.keys().copied().collect());
        let comp_task_ids: HashSet<u64> = self.task_comps.keys().copied().collect();

        let num_all = task_ids.len();
        let num_done = ctx.with_model(|task| task.map.values().filter(|t| t.done).count());
        let num_todo = num_all - num_done;

        self.summary.set_text(&format!(
            "All: {}  Todo: {}  Done: {}",
            num_all, num_todo, num_done
        ));

        // Remove components that are no longer in the model
        for task_id in comp_task_ids.difference(&task_ids) {
            self.vbox
                .remove(&self.task_comps.get(task_id).unwrap().widget());
            self.task_comps.remove(task_id);
        }

        // Create components for new tasks
        for task_id in task_ids.difference(&comp_task_ids).copied() {
            let c = ctx.create_child(
                move |tasks| tasks.map.get_mut(&task_id).unwrap(),
                ctx.clone(),
            );
            self.vbox.append(&c.widget());
            self.task_comps.insert(task_id, c);
        }
    }
}
