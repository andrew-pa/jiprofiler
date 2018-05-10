
use view::Resources;

use runic::*;
use winit::*;

use std::iter::{FromIterator, repeat};
use std::cell::RefCell;

//enum MenuItem {
//    Action { display_name: String, action: Box<FnOnce()> },
//    Divider,
//    SubMenu(Menu)
//}
//

struct Menu {
    location: Point,
    //callback: Box<RefCell<FnMut(usize)>>,
    name: &'static str,
    items: Vec<(String, Option<TextLayout>)>,
    bounds: Option<Rect>,
    selected: isize
}

pub struct MenuContext {
    open_menus: Vec<Menu>,
    active: isize
}

impl MenuContext {
    pub fn new() -> MenuContext {
        MenuContext {
            open_menus: Vec::new(),
            active: -1
        }
    }

    pub fn event(&mut self, e: &WindowEvent) -> Option<(&'static str, usize)> {
        match e {
            &WindowEvent::CursorMoved { position: (x,y), .. } => {
                let p = Point::xy(x as f32, y as f32);
                self.active = -1;
                for (i,m) in self.open_menus.iter_mut().enumerate() {
                    if let Some(bnd) = m.bounds {
                        if bnd.contains(p) {
                            self.active = i as isize;
                            m.selected = -1;
                            let mut current_item_y = m.location.y;
                            for (j, &(_, ref ly)) in m.items.iter().enumerate() {
                                if let Some(item_height) = ly.as_ref().map(|i| i.bounds().h) {
                                    if p.y > current_item_y && p.y < current_item_y + item_height {
                                        m.selected = j as isize;
                                        break;
                                    }
                                    current_item_y += item_height;
                                }
                            }
                            break;
                        }
                    }
                }
                None
            },
            &WindowEvent::MouseInput { state, button, .. } => {
                let rv = if self.active >= 0 && state == ElementState::Released && button == MouseButton::Left {
                        let m = &mut self.open_menus[self.active as usize];
                        if m.selected >= 0 {
                            self.active = -1;
                            Some((m.name, m.selected as usize))
                        } else { None }
                } else { None };
                if self.active == -1 { self.open_menus.clear(); }
                rv
            },
            _ => None
        }
    }

    pub fn paint(&mut self, rx: &mut RenderContext, res: &Resources) {
        for (i,menu) in self.open_menus.iter_mut().enumerate() {
            if let None = menu.bounds {
                let mut bnd = Rect::pnwh(menu.location, 4.0, 4.0);
                bnd.x -= 4.0; bnd.y -= 4.0;
                for &mut (ref s, ref mut oly) in menu.items.iter_mut() {
                    let lyb = oly.get_or_insert_with(||rx.new_text_layout(&s, &res.font, 256.0, 64.0).expect("create text layout for menu"))
                        .bounds();
                    bnd.w = bnd.w.max(lyb.w+4.0);
                    bnd.h += lyb.h;
                }
                menu.bounds = Some(bnd);
            }
            let bnd = menu.bounds.unwrap();
            rx.set_color(Color::rgb(0.8, 0.8, 0.8));
            rx.fill_rect(bnd);
            rx.set_color(Color::rgb(0.0, 0.0, 0.0));
            rx.stroke_rect(bnd, 2.0);
            let mut p = Point::xy(bnd.x+2.0, bnd.y+2.0);
            let mut i = 0;
            for &(_, ref ly) in menu.items.iter() {
                let lyr = ly.as_ref().unwrap();
                let b = lyr.bounds();
                if i == menu.selected {
                    rx.set_color(Color::rgb(0.4, 0.45, 0.4));
                    rx.fill_rect(Rect::xywh(p.x, p.y, bnd.w-4.0, b.h)); 
                    rx.set_color(Color::rgb(0.0, 0.0, 0.0));
                }
                rx.draw_text_layout(p, lyr);
                p.y += b.h;
                i+=1;
            }
        }
    }

    pub fn popup(&mut self, items: Vec<&str>, p: Point, name: &'static str) {
        self.active = self.open_menus.len() as isize;
        self.open_menus.push(Menu {
            location: p, 
            name: name,
            items: items.iter().map(|&s| String::from(s)).zip(repeat(None)).collect(),
            bounds: None,
            selected: -1
        });
    }
}


