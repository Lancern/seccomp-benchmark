pub(crate) struct DeferWrapper<F>
    where F: FnOnce() {
    action: Option<F>,
}

impl<F> DeferWrapper<F>
    where F: FnOnce() +  {
    pub(crate) fn new(action: F) -> Self {
        DeferWrapper {
            action: Some(action),
        }
    }
}

impl<F> Drop for DeferWrapper<F>
    where F: FnOnce() {
    fn drop(&mut self) {
        match self.action.take() {
            Some(f) => f(),
            None => ()
        }
    }
}

pub(crate) fn defer<F>(action: F) -> DeferWrapper<F>
    where F: FnOnce() {
    DeferWrapper::new(action)
}

pub(crate) fn bitcast<TFrom, TTo>(value: TFrom) -> TTo
    where TFrom: Copy,
          TTo: Copy {
    if std::mem::size_of::<TFrom>() != std::mem::size_of::<TTo>() {
        panic!("Sizes of TFrom and TTto are different.");
    }

    unsafe {
        *((&value as *const TFrom) as *const TTo)
    }
}
