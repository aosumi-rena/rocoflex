use super::*;
use super::super::crt_query_owned::{SourceCrtQueryOwned, SourceCrtQueryOwnedBindings, SourceCrtQueryCursor};

#[must_use = "retain SAME handler/lock/actual entry/epilogue/RET and native resources with query P/S"]
pub struct SourceCrtHandlerQueryHistory<D, P, S, A, R, N> {
    pub handler: D, pub actual_entry: A, pub actual_epilogue: R, pub actual_return: N,
    pub retired: SourceCrtNativeRetired,
    pub bindings: SourceCrtHandlerFrameBindings<D, P, S, A>,
    pub entry: SourceCrtHandlerActualEntry, pub call: SourceCrtCallocCall, pub normalized_eax: u32,
}
impl<D, P, S, A, R, N> SourceCrtHandlerFrameReturned<D, P, S, A, R, N> {

    pub fn into_owned_query(self, bindings: SourceCrtQueryOwnedBindings<P, S>)
        -> Result<SourceCrtQueryOwned<P, S, SourceCrtHandlerQueryHistory<D, P, S, A, R, N>>, Self> {
        let current = (self.owner.bindings.current_prefix)(&self.owner.prefix, &self.owner.services);
        if current.frame != self.retired.after || current.rbx != self.owner.request.required_bytes_rbx
            || current.rdi != self.owner.entry.entry.rdi
            || (bindings.actual_frame)(&self.owner.prefix, &self.owner.services) != current.frame {
            return Err(self);
        }
        let (source, retained) = self.into_source_and_retained()?;
        let SourceCrtHandlerFrameRetained { handler, prefix, services, actual_entry, actual_epilogue,
            actual_return, retired, bindings: handler_bindings, entry, call, normalized_eax } = retained;
        let history = SourceCrtHandlerQueryHistory { handler, actual_entry, actual_epilogue,
            actual_return, retired, bindings: handler_bindings, entry, call, normalized_eax };
        Ok(SourceCrtQueryOwned::from_same_cursor_and_history(SourceCrtQueryCursor::Calloc(source),
            prefix, services, history, bindings))
    }
}
