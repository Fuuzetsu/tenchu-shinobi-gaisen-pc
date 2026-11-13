use super::project::Project;

#[derive(Default)]
pub struct History {
    undo: Vec<Project>,
    redo: Vec<Project>,
    stroke: Option<Project>,
}

impl History {
    pub fn begin(&mut self, project: &Project) {
        self.stroke.get_or_insert_with(|| project.clone());
    }

    pub fn commit(&mut self, project: &mut Project) -> bool {
        let Some(before) = self.stroke.take().filter(|before| before != project) else {
            return false;
        };
        if self.undo.len() == 128 {
            self.undo.remove(0);
        }
        if before.mission != project.mission
            || before.actors != project.actors
            || before.events != project.events
        {
            project.mission.records.clear();
        }
        self.undo.push(before);
        self.redo.clear();
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, project: &mut Project) -> bool {
        self.commit(project);
        let Some(before) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(project, before));
        true
    }

    pub fn redo(&mut self, project: &mut Project) -> bool {
        self.commit(project);
        let Some(after) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(project, after));
        true
    }
}
