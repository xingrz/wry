// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
  cell::{Cell, UnsafeCell},
  path::PathBuf,
  rc::Rc,
};

use gtk::{glib::GString, prelude::*};
use webkit2gtk::WebView;

use crate::DragDropEvent;

struct DragDropController {
  paths: UnsafeCell<Option<Vec<PathBuf>>>,
  has_entered: Cell<bool>,
  position: Cell<(i32, i32)>,
  handler: Box<dyn Fn(DragDropEvent) -> bool>,
}

impl DragDropController {
  fn new(handler: Box<dyn Fn(DragDropEvent) -> bool>) -> Self {
    Self {
      handler,
      paths: UnsafeCell::new(None),
      has_entered: Cell::new(false),
      position: Cell::new((0, 0)),
    }
  }

  fn store_paths(&self, paths: Vec<PathBuf>) {
    unsafe { *self.paths.get() = Some(paths) };
  }

  fn take_paths(&self) -> Option<Vec<PathBuf>> {
    unsafe { &mut *self.paths.get() }.take()
  }

  fn store_position(&self, position: (i32, i32)) {
    self.position.replace(position);
  }

  fn enter(&self) {
    self.has_entered.set(true);
  }

  fn has_entered(&self) -> bool {
    self.has_entered.get()
  }

  fn leave(&self) {
    self.has_entered.set(false);
  }

  fn call(&self, event: DragDropEvent) -> bool {
    (self.handler)(event)
  }
}

pub(crate) fn connect_drag_event(webview: &WebView, handler: Box<dyn Fn(DragDropEvent) -> bool>) {
  let controller = Rc::new(DragDropController::new(handler));

  {
    let controller = controller.clone();
    webview.connect_drag_data_received(move |_, _, _, _, data, info, _| {
      println!("drag data received info: {}", info);
      if info == 2 {
        let uris = data.uris();
        let paths = uris.iter().map(path_buf_from_uri).collect::<Vec<_>>();
        controller.enter();
        println!("entered with paths: {:?}", paths);
        controller.call(DragDropEvent::Enter {
          paths: paths.clone(),
          position: controller.position.get(),
        });
        controller.store_paths(paths);
      }
    });
  }

  // {
  //   let controller = controller.clone();
  //   webview.connect_drag_motion(move |_, _, x, y, _| {
  //     println!("drag motion at x: {}, y: {}", x, y);
  //     if controller.has_entered() {
  //       controller.call(DragDropEvent::Over { position: (x, y) });
  //     } else {
  //       controller.store_position((x, y));
  //     }
  //     false
  //   });
  // }

  {
    let controller = controller.clone();
    webview.connect_drag_drop(move |_, _, x, y, _| {
      println!("drag drop at x: {}, y: {}", x, y);
      if controller.has_entered() {
        println!("has entered");
        if let Some(paths) = controller.take_paths() {
          println!("has paths");
          controller.leave();
          return controller.call(DragDropEvent::Drop {
            paths,
            position: (x, y),
          });
        }
      }

      false
    });
  }

  webview.connect_drag_leave(move |_, _, time| {
    println!("drag leave time: {}", time);
    if time == 0 {
      controller.leave();
      controller.call(DragDropEvent::Leave);
    }
  });

  webview.connect_drag_begin(move |_, _| {
    println!("drag begin");
  });

  webview.connect_drag_end(move |_, _| {
    println!("drag end");
  });

  webview.connect_drag_failed(move |_, _, _| {
    println!("drag failed");
    // if listener_ref.0(FileDropEvent::Cancelled) {
    //   glib::Propagation::Stop
    // } else {
    gtk::glib::Propagation::Proceed
    // }
  });
}

fn path_buf_from_uri(gstr: &GString) -> PathBuf {
  let path = gstr.as_str();
  let path = path.strip_prefix("file://").unwrap_or(path);
  let path = percent_encoding::percent_decode(path.as_bytes())
    .decode_utf8_lossy()
    .to_string();
  PathBuf::from(path)
}
