use crate::wayland::toplevel::{Toplevel, ToplevelEvent};
use crate::wayland::LazyGlobal;
use smithay_client_toolkit::environment::{Environment, GlobalHandler};
use std::cell::RefCell;
use std::rc;
use std::rc::Rc;
use tracing::warn;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{Attached, DispatchData};
use wayland_protocols::wlr::unstable::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};


struct ToplevelHandlerInner {
    manager: LazyGlobal<ZwlrForeignToplevelManagerV1>,
    registry: Option<Attached<WlRegistry>>,
    toplevels: Vec<Toplevel>,
}

impl ToplevelHandlerInner {
    fn new() -> Self {
        let toplevels = vec![];

        Self {
            registry: None,
            manager: LazyGlobal::Unknown,
            toplevels,
        }
    }
}

pub struct ToplevelHandler {
    inner: Rc<RefCell<ToplevelHandlerInner>>,
    status_listeners: Rc<RefCell<Vec<rc::Weak<RefCell<ToplevelStatusCallback>>>>>,
}

impl ToplevelHandler {
    pub fn init() -> Self {
        let inner = Rc::new(RefCell::new(ToplevelHandlerInner::new()));

        Self {
            inner,
            status_listeners: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl GlobalHandler<ZwlrForeignToplevelManagerV1> for ToplevelHandler {
    fn created(
        &mut self,
        registry: Attached<WlRegistry>,
        id: u32,
        version: u32,
        _ddata: DispatchData,
    ) {
        let mut inner = RefCell::borrow_mut(&self.inner);
        if inner.registry.is_none() {
            inner.registry = Some(registry);
        }
        if let LazyGlobal::Unknown = inner.manager {
            inner.manager = LazyGlobal::Seen { id, version }
        } else {
            warn!(
                "Compositor advertised zwlr_foreign_toplevel_manager_v1 multiple times, ignoring."
            )
        }
    }

    fn get(&self) -> Option<Attached<ZwlrForeignToplevelManagerV1>> {
        let mut inner = RefCell::borrow_mut(&self.inner);

        match inner.manager {
            LazyGlobal::Bound(ref mgr) => Some(mgr.clone()),
            LazyGlobal::Unknown => None,
            LazyGlobal::Seen { id, version } => {
                let registry = inner.registry.as_ref().expect("Failed to get registry");
                // current max protocol version = 3
                let version = std::cmp::min(version, 3);
                let manager = registry.bind::<ZwlrForeignToplevelManagerV1>(version, id);

                {
                    let inner = self.inner.clone();
                    let status_listeners = self.status_listeners.clone();

                    manager.quick_assign(move |_, event, _ddata| {
                        let mut inner = RefCell::borrow_mut(&inner);
                        let status_listeners = status_listeners.clone();

                        match event {
                            zwlr_foreign_toplevel_manager_v1::Event::Toplevel {
                                toplevel: handle,
                            } => {
                                let toplevel = Toplevel::init(handle.clone(), move |event, ddata| {
                                    notify_status_listeners(
                                        &handle,
                                        event,
                                        ddata,
                                        &status_listeners,
                                    );
                                });

                                inner.toplevels.push(toplevel);
                            }
                            zwlr_foreign_toplevel_manager_v1::Event::Finished => {}
                            _ => unreachable!(),
                        }
                    });
                }

                inner.manager = LazyGlobal::Bound((*manager).clone());
                Some((*manager).clone())
            }
        }
    }
}

type ToplevelStatusCallback =
    dyn FnMut(ZwlrForeignToplevelHandleV1, ToplevelEvent, DispatchData) + 'static;

/// Notifies the callbacks of an event on the toplevel
fn notify_status_listeners(
    toplevel: &ZwlrForeignToplevelHandleV1,
    event: ToplevelEvent,
    mut ddata: DispatchData,
    listeners: &RefCell<Vec<rc::Weak<RefCell<ToplevelStatusCallback>>>>,
) {
    listeners.borrow_mut().retain(|lst| {
        if let Some(cb) = rc::Weak::upgrade(lst) {
            (cb.borrow_mut())(toplevel.clone(), event.clone(), ddata.reborrow());
            true
        } else {
            false
        }
    })
}

pub struct ToplevelStatusListener {
    _cb: Rc<RefCell<ToplevelStatusCallback>>,
}

pub trait ToplevelHandling {
    fn listen<F>(&mut self, f: F) -> ToplevelStatusListener
    where
        F: FnMut(ZwlrForeignToplevelHandleV1, ToplevelEvent, DispatchData) + 'static;
}

impl ToplevelHandling for ToplevelHandler {
    fn listen<F>(&mut self, f: F) -> ToplevelStatusListener
    where
        F: FnMut(ZwlrForeignToplevelHandleV1, ToplevelEvent, DispatchData) + 'static,
    {
        let rc = Rc::new(RefCell::new(f)) as Rc<_>;
        self.status_listeners.borrow_mut().push(Rc::downgrade(&rc));
        ToplevelStatusListener { _cb: rc }
    }
}

pub fn listen_for_toplevels<E, F>(env: Environment<E>, f: F) -> ToplevelStatusListener
where
    E: ToplevelHandling,
    F: FnMut(ZwlrForeignToplevelHandleV1, ToplevelEvent, DispatchData) + 'static,
{
    env.with_inner(move |inner| ToplevelHandling::listen(inner, f))
}
