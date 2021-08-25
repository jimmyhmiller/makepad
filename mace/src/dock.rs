use {
    crate::{
        generational::{Arena, Id},
        list_logic::ItemId,
        splitter::{self, Splitter},
        tab_bar::{self, TabBar},
    },
    makepad_render::*,
};

pub struct Dock {
    panels: Arena<Panel>,
    panel_id_stack: Vec<PanelId>,
}

impl Dock {
    pub fn new(_cx: &mut Cx) -> Dock {
        Dock {
            panels: Arena::new(),
            panel_id_stack: Vec::new(),
        }
    }

    pub fn begin_splitter(&mut self, cx: &mut Cx, panel_id: PanelId) -> Result<(), ()> {
        if !self.panels.contains(panel_id) {
            self.panels.insert(panel_id, Panel::Splitter(Splitter::new(cx)));
        }
        let splitter = self.panels[panel_id].as_splitter_mut();
        splitter.begin(cx)?;
        self.panel_id_stack.push(panel_id);
        Ok(())
    }

    pub fn middle_splitter(&mut self, cx: &mut Cx) {
        let panel_id = *self.panel_id_stack.last().unwrap();
        let splitter = self.panels[panel_id].as_splitter_mut();
        splitter.middle(cx);
    }

    pub fn end_splitter(&mut self, cx: &mut Cx) {
        let panel_id = self.panel_id_stack.pop().unwrap();
        let splitter = self.panels[panel_id].as_splitter_mut();
        splitter.end(cx);
    }

    pub fn begin_tab_bar(&mut self, cx: &mut Cx, panel_id: PanelId) -> Result<(), ()> {
        if !self.panels.contains(panel_id) {
            self.panels.insert(panel_id, Panel::TabBar(TabBar::new(cx)));
        }
        let tab_bar = self.panels[panel_id].as_tab_bar_mut();
        tab_bar.begin(cx)?;
        self.panel_id_stack.push(panel_id);
        Ok(())
    }

    pub fn end_tab_bar(&mut self, cx: &mut Cx) {
        let panel_id = self.panel_id_stack.pop().unwrap();
        let tab_bar = self.panels[panel_id].as_tab_bar_mut();
        tab_bar.end(cx);
    }

    pub fn tab(&mut self, cx: &mut Cx, tab_id: ItemId, name: &str) {
        let panel_id = *self.panel_id_stack.last().unwrap();
        let tab_bar = self.panels[panel_id].as_tab_bar_mut();
        tab_bar.tab(cx, tab_id, name);
    }

    pub fn get_or_create_splitter(&mut self, cx: &mut Cx, panel_id: PanelId) -> &mut Splitter {
        if !self.panels.contains(panel_id) {
            self.panels.insert(panel_id, Panel::Splitter(Splitter::new(cx)));
        }
        self.panels[panel_id].as_splitter_mut()
    }

    pub fn get_or_create_tab_bar(&mut self, cx: &mut Cx, panel_id: PanelId) -> &mut TabBar {
        if !self.panels.contains(panel_id) {
            self.panels.insert(panel_id, Panel::TabBar(TabBar::new(cx)));
        }
        self.panels[panel_id].as_tab_bar_mut()
    }

    pub fn handle_event(
        &mut self,
        cx: &mut Cx,
        event: &mut Event,
        dispatch_action: &mut dyn FnMut(Action),
    ) {
        for (_, panel) in &mut self.panels {
            match panel {
                Panel::Splitter(splitter) => {
                    let mut actions = Vec::new();
                    splitter.handle_event(cx, event, &mut |action| actions.push(action));
                    for action in actions {
                        match action {
                            splitter::Action::Redraw => {
                                cx.redraw_child_area(Area::All);
                            }
                        }
                    }
                }
                Panel::TabBar(tab_bar) => {
                    let mut actions = Vec::new();
                    tab_bar.handle_event(cx, event, &mut |action| actions.push(action));
                    for action in actions {
                        match action {
                            tab_bar::Action::TabWasPressed(item_id) => {
                                dispatch_action(Action::TabWasPressed(item_id))
                            }
                        }
                    }
                }
            }
        }
    }
}

pub type PanelId = Id;

enum Panel {
    Splitter(Splitter),
    TabBar(TabBar),
}

impl Panel {
    fn as_splitter_mut(&mut self) -> &mut Splitter {
        match self {
            Panel::Splitter(splitter) => splitter,
            _ => panic!(),
        }
    }

    fn as_tab_bar_mut(&mut self) -> &mut TabBar {
        match self {
            Panel::TabBar(tab_bar) => tab_bar,
            _ => panic!(),
        }
    }
}

pub enum Action {
    TabWasPressed(ItemId),
}