#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtPointerSlot {
    Slot8611f8, Slot861040, Slot861c68, Slot861c70,
    Slot861c78, Slot861c80, Matherr861c88, Slot861048,
}
impl SourceCrtPointerSlot {
    pub fn source_rva(self) -> u32 {
        match self {
            Self::Slot8611f8 => 0x8611f8, Self::Slot861040 => 0x861040,
            Self::Slot861c68 => 0x861c68, Self::Slot861c70 => 0x861c70,
            Self::Slot861c78 => 0x861c78, Self::Slot861c80 => 0x861c80,
            Self::Matherr861c88 => 0x861c88, Self::Slot861048 => 0x861048,
        }
    }
}
const ORDER: [SourceCrtPointerSlot; 8] = [
    SourceCrtPointerSlot::Slot8611f8, SourceCrtPointerSlot::Slot861040,
    SourceCrtPointerSlot::Slot861c68, SourceCrtPointerSlot::Slot861c70,
    SourceCrtPointerSlot::Slot861c78, SourceCrtPointerSlot::Slot861c80,
    SourceCrtPointerSlot::Matherr861c88, SourceCrtPointerSlot::Slot861048,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtPointerStore {
    pub ordinal: u8,
    pub target: SourceCrtPointerSlot,
    pub encoded_word: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourcePointerInitializerRequest {

    SecurityWord820dd8,
    Store(SourceCrtPointerStore),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePointerInitializerCompletion {
    source_security_word: u64,
    stores: [SourceCrtPointerStore; 8],
}
impl SourcePointerInitializerCompletion {
    pub fn source_security_word(&self) -> u64 { self.source_security_word }
    pub fn ordered_stores(&self) -> &[SourceCrtPointerStore; 8] { &self.stores }
    pub fn initial_encoded_matherr_word(&self) -> u64 { self.stores[6].encoded_word }

    pub fn return_al(&self) -> u8 { 1 }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourcePointerInitializerProgress {
    Request(SourcePointerInitializerRequest),
    Complete(SourcePointerInitializerCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourcePointerInitializerError {
    UnexpectedSecurityWord { pending: SourcePointerInitializerProgress, supplied_word: u64 },
    UnexpectedStoreReceipt { pending: SourcePointerInitializerProgress, supplied: SourceCrtPointerStore },
    StoreBindingRequired { expected: SourceCrtPointerStore, supplied: SourceCrtPointerStore },
}
impl SourcePointerInitializerError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::UnexpectedSecurityWord { .. } => "567d24 supplied security word does not match pending source read",
            Self::UnexpectedStoreReceipt { .. } => "567d24 actual store receipt does not match its pending stage",
            Self::StoreBindingRequired { .. } => "567d24 store receipt has a different ordinal/slot/saved-security word",
        }
    }
}
pub struct SourcePointerInitializerServiceFailure<E> {
    pub cause: E,
    pub pending: SourcePointerInitializerRequest,
    pub completed_stores: [Option<SourceCrtPointerStore>; 8],
}
pub struct SourcePointerInitializerRejectedFailure<E> {
    pub cause: E,
    pub completion: SourcePointerInitializerCompletion,
}

pub struct SourceCrtPointerInitializer567d24 {
    security: Option<u64>,
    stores: [Option<SourceCrtPointerStore>; 8],
    progress: SourcePointerInitializerProgress,
    failed_service: bool,
}
impl SourceCrtPointerInitializer567d24 {
    pub fn begin_serial() -> Self {
        Self { security: None, stores: [None; 8], failed_service: false,
            progress: SourcePointerInitializerProgress::Request(SourcePointerInitializerRequest::SecurityWord820dd8) }
    }
    pub fn progress(&self) -> SourcePointerInitializerProgress { self.progress }
    pub fn request(&self) -> Option<SourcePointerInitializerRequest> {
        if self.failed_service { return None; }
        self.awaiting_actual_completion()
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourcePointerInitializerRequest> {
        match self.progress { SourcePointerInitializerProgress::Request(request) => Some(request), _ => None }
    }
    pub fn supplied_security_word(&self) -> Option<u64> { self.security }
    pub fn completed_stores(&self) -> &[Option<SourceCrtPointerStore>; 8] { &self.stores }
    pub fn supply_security_word(&mut self, word: u64) -> Result<SourcePointerInitializerProgress, SourcePointerInitializerError> {
        self.failed_service = true;
        if self.progress != SourcePointerInitializerProgress::Request(SourcePointerInitializerRequest::SecurityWord820dd8) {
            return Err(SourcePointerInitializerError::UnexpectedSecurityWord { pending: self.progress, supplied_word: word });
        }
        self.security = Some(word);
        self.progress = SourcePointerInitializerProgress::Request(SourcePointerInitializerRequest::Store(
            SourceCrtPointerStore { ordinal: 0, target: ORDER[0], encoded_word: word }));
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn accept_store_completion(&mut self, supplied: SourceCrtPointerStore)
        -> Result<SourcePointerInitializerProgress, SourcePointerInitializerError>
    {
        self.failed_service = true;
        let SourcePointerInitializerProgress::Request(SourcePointerInitializerRequest::Store(expected)) = self.progress
            else { return Err(SourcePointerInitializerError::UnexpectedStoreReceipt { pending: self.progress, supplied }); };
        if supplied != expected { return Err(SourcePointerInitializerError::StoreBindingRequired { expected, supplied }); }

        let index = usize::from(expected.ordinal);
        self.stores[index] = Some(supplied);
        if index + 1 == ORDER.len() {

            let stores = core::array::from_fn(|ordinal| SourceCrtPointerStore {
                ordinal: ordinal as u8, target: ORDER[ordinal], encoded_word: expected.encoded_word,
            });
            self.progress = SourcePointerInitializerProgress::Complete(SourcePointerInitializerCompletion {
                source_security_word: expected.encoded_word, stores,
            });
        } else {
            self.progress = SourcePointerInitializerProgress::Request(SourcePointerInitializerRequest::Store(
                SourceCrtPointerStore { ordinal: (index + 1) as u8, target: ORDER[index + 1], encoded_word: expected.encoded_word }));
        }
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E)
        -> Result<SourcePointerInitializerServiceFailure<E>, SourcePointerInitializerRejectedFailure<E>>
    {
        match self.progress {
            SourcePointerInitializerProgress::Request(pending) => {
                self.failed_service = true;
                Ok(SourcePointerInitializerServiceFailure { cause, pending, completed_stores: self.stores })
            }
            SourcePointerInitializerProgress::Complete(completion) => Err(SourcePointerInitializerRejectedFailure { cause, completion }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtTableReadRole { Initializer, RollbackInitializer, Uninitializer }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtTableRead {
    pub ordinal: u8,
    pub role: SourceCrtTableReadRole,
    pub actual_address: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtTableCallRole { Initialize, Rollback }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtTableCall {
    pub ordinal: u8,
    pub role: SourceCrtTableCallRole,

    pub actual_target: u64,
    pub type_hash_r10: u64,

    pub ecx_override: Option<u32>,

    pub expected_rbx: u64,
    pub expected_rsi: u64,
    pub expected_rdi: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtTableRequest {
    Read(SourceCrtTableRead),

    Guard(SourceCrtTableCall),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtTableReceipt {
    Word { read: SourceCrtTableRead, word: u64 },
    GuardReturned {
        call: SourceCrtTableCall,
        return_al: u8,
        actual_rbx: u64,
        actual_rsi: u64,
        actual_rdi: u64,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtTableCompletion {
    actual_begin: u64,
    actual_end: u64,
    return_al: u8,
    initializer_words: [Option<u64>; 16],
    initializer_returns: [Option<u8>; 16],
    rollback_initializer_words: [Option<u64>; 16],
    uninitializer_words: [Option<u64>; 16],
    rollback_returns: [Option<u8>; 16],
}
impl SourceCrtTableCompletion {
    pub fn actual_begin(&self) -> u64 { self.actual_begin }
    pub fn actual_end(&self) -> u64 { self.actual_end }

    pub fn return_al(&self) -> u8 { self.return_al }
    pub fn initializer_words(&self) -> &[Option<u64>; 16] { &self.initializer_words }
    pub fn initializer_returns(&self) -> &[Option<u8>; 16] { &self.initializer_returns }
    pub fn rollback_initializer_words(&self) -> &[Option<u64>; 16] { &self.rollback_initializer_words }
    pub fn uninitializer_words(&self) -> &[Option<u64>; 16] { &self.uninitializer_words }
    pub fn rollback_returns(&self) -> &[Option<u8>; 16] { &self.rollback_returns }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtTableProgress {
    Request(SourceCrtTableRequest),
    Complete(SourceCrtTableCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtTableError {
    RelocatedTableRangeRequired { supplied_begin: u64 },
    ReceiptBindingRequired { pending: SourceCrtTableProgress, actual: SourceCrtTableReceipt },
    ActualNonvolatileContinuationRequired { pending: SourceCrtTableCall, actual: SourceCrtTableReceipt },
}
impl SourceCrtTableError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::RelocatedTableRangeRequired { .. } => "567e0c relocated sixteen-record table range overflows supplied address width",
            Self::ReceiptBindingRequired { .. } => "590788 actual table/guard receipt does not match its pending source operation",
            Self::ActualNonvolatileContinuationRequired { .. } => "590788 actual guarded return changed source nonvolatile traversal registers",
        }
    }
}
pub struct SourceCrtTableServiceFailure<E> {
    pub cause: E,

    pub pending: SourceCrtTableRequest,
}
pub struct SourceCrtTableRejectedFailure<E> {
    pub cause: E,
    pub completion: SourceCrtTableCompletion,
}

pub struct SourceCrtTableInitializer567e0c {
    begin: u64,
    end: u64,
    initializer_words: [Option<u64>; 16],
    initializer_returns: [Option<u8>; 16],
    rollback_initializer_words: [Option<u64>; 16],
    uninitializer_words: [Option<u64>; 16],
    rollback_returns: [Option<u8>; 16],
    progress: SourceCrtTableProgress,
    failed_service: bool,
}
impl SourceCrtTableInitializer567e0c {
    pub fn begin_with_actual_table(actual_begin: u64) -> Result<Self, SourceCrtTableError> {
        let Some(end) = actual_begin.checked_add(0x100) else {
            return Err(SourceCrtTableError::RelocatedTableRangeRequired { supplied_begin: actual_begin });
        };
        Ok(Self {
            begin: actual_begin, end, initializer_words: [None; 16], initializer_returns: [None; 16],
            rollback_initializer_words: [None; 16], uninitializer_words: [None; 16], rollback_returns: [None; 16],
            progress: SourceCrtTableProgress::Request(SourceCrtTableRequest::Read(SourceCrtTableRead {
                ordinal: 0, role: SourceCrtTableReadRole::Initializer, actual_address: actual_begin,
            })), failed_service: false,
        })
    }
    pub fn progress(&self) -> SourceCrtTableProgress { self.progress }
    pub fn initializer_words(&self) -> &[Option<u64>; 16] { &self.initializer_words }
    pub fn initializer_returns(&self) -> &[Option<u8>; 16] { &self.initializer_returns }
    pub fn rollback_initializer_words(&self) -> &[Option<u64>; 16] { &self.rollback_initializer_words }
    pub fn uninitializer_words(&self) -> &[Option<u64>; 16] { &self.uninitializer_words }
    pub fn rollback_returns(&self) -> &[Option<u8>; 16] { &self.rollback_returns }
    pub fn request(&self) -> Option<SourceCrtTableRequest> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtTableRequest> {
        match self.progress { SourceCrtTableProgress::Request(request) => Some(request), _ => None }
    }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtTableReceipt)
        -> Result<SourceCrtTableProgress, SourceCrtTableError>
    {
        self.failed_service = true;
        match (self.progress, actual) {
            (SourceCrtTableProgress::Request(SourceCrtTableRequest::Read(expected)),
                SourceCrtTableReceipt::Word { read, word }) if expected == read => {
                let index = usize::from(expected.ordinal);
                match expected.role {
                    SourceCrtTableReadRole::Initializer => {
                        self.initializer_words[index] = Some(word);
                        if word == 0 { self.advance_forward(index); }
                        else { self.call(index, SourceCrtTableCallRole::Initialize, word); }
                    }
                    SourceCrtTableReadRole::RollbackInitializer => {
                        self.rollback_initializer_words[index] = Some(word);
                        if word == 0 { self.advance_rollback(index); }
                        else { self.read(index, SourceCrtTableReadRole::Uninitializer); }
                    }
                    SourceCrtTableReadRole::Uninitializer => {
                        self.uninitializer_words[index] = Some(word);
                        if word == 0 { self.advance_rollback(index); }
                        else { self.call(index, SourceCrtTableCallRole::Rollback, word); }
                    }
                }
            }
            (SourceCrtTableProgress::Request(SourceCrtTableRequest::Guard(expected)),
                SourceCrtTableReceipt::GuardReturned { call, return_al, actual_rbx, actual_rsi, actual_rdi }) if expected == call => {
                if (actual_rbx, actual_rsi, actual_rdi) != (expected.expected_rbx, expected.expected_rsi, expected.expected_rdi) {
                    return Err(SourceCrtTableError::ActualNonvolatileContinuationRequired { pending: expected, actual });
                }
                let index = usize::from(expected.ordinal);
                match expected.role {
                    SourceCrtTableCallRole::Initialize => {
                        self.initializer_returns[index] = Some(return_al);
                        if return_al != 0 { self.advance_forward(index); }
                        else { self.advance_rollback(index); }
                    }
                    SourceCrtTableCallRole::Rollback => {
                        self.rollback_returns[index] = Some(return_al);
                        self.advance_rollback(index);
                    }
                }
            }
            _ => return Err(SourceCrtTableError::ReceiptBindingRequired { pending: self.progress, actual }),
        }
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E)
        -> Result<SourceCrtTableServiceFailure<E>, SourceCrtTableRejectedFailure<E>>
    {
        match self.progress {
            SourceCrtTableProgress::Request(pending) => {
                self.failed_service = true;
                Ok(SourceCrtTableServiceFailure { cause, pending })
            }
            SourceCrtTableProgress::Complete(completion) => Err(SourceCrtTableRejectedFailure { cause, completion }),
        }
    }
    fn read(&mut self, index: usize, role: SourceCrtTableReadRole) {
        let offset = index as u64 * 16 + if role == SourceCrtTableReadRole::Uninitializer { 8 } else { 0 };
        self.progress = SourceCrtTableProgress::Request(SourceCrtTableRequest::Read(SourceCrtTableRead {
            ordinal: index as u8, role, actual_address: self.begin + offset,
        }));
    }
    fn call(&mut self, index: usize, role: SourceCrtTableCallRole, target: u64) {
        let rollback = role == SourceCrtTableCallRole::Rollback;
        self.progress = SourceCrtTableProgress::Request(SourceCrtTableRequest::Guard(SourceCrtTableCall {
            ordinal: index as u8, role, actual_target: target,
            type_hash_r10: if rollback { 0x9799_a675_3e59_3b70 } else { 0xdf94_9ec4_5c5c_a270 },
            ecx_override: if rollback { Some(0) } else { None },
            expected_rbx: self.begin + index as u64 * 16 + if rollback { 8 } else { 0 },
            expected_rsi: self.end, expected_rdi: self.begin,
        }));
    }
    fn advance_forward(&mut self, index: usize) {
        if index == 15 { self.complete(1); }
        else { self.read(index + 1, SourceCrtTableReadRole::Initializer); }
    }
    fn advance_rollback(&mut self, index: usize) {
        if index == 0 { self.complete(0); }
        else { self.read(index - 1, SourceCrtTableReadRole::RollbackInitializer); }
    }
    fn complete(&mut self, return_al: u8) {
        self.progress = SourceCrtTableProgress::Complete(SourceCrtTableCompletion {
            actual_begin: self.begin, actual_end: self.end, return_al,
            initializer_words: self.initializer_words, initializer_returns: self.initializer_returns,
            rollback_initializer_words: self.rollback_initializer_words,
            uninitializer_words: self.uninitializer_words, rollback_returns: self.rollback_returns,
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtStartupCall {

    GeneralIsa561dd4,
    Resources564ccc,
    FailureCleanup564cf4,
}
impl SourceCrtStartupCall {
    pub fn source_rva(self) -> u32 {
        match self { Self::GeneralIsa561dd4 => 0x561dd4, Self::Resources564ccc => 0x564ccc,
            Self::FailureCleanup564cf4 => 0x564cf4 }
    }
    pub fn ecx_override(self) -> Option<u32> {
        if self == Self::FailureCleanup564cf4 { Some(0) } else { None }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtStartupRequest {
    ReadModeFlag85f430,
    StoreModeFlag85f430 { value: u8 },

    Call(SourceCrtStartupCall),
    InitializerTable { actual_begin: u64, actual_end: u64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtStartupReceipt {
    ModeFlag { value: u8 },
    ModeFlagStored { value: u8 },
    CallReturned { call: SourceCrtStartupCall, actual_rax: u64, actual_rbx: u64 },
    InitializerTableReturned { completion: SourceCrtTableCompletion, actual_rbx: u64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStartupPrefix {
    source_entry_ecx: u32,
    original_mode_flag: Option<u8>,
    stored_mode_flag: Option<u8>,
    isa_return_rax: Option<u64>,
    resources_return_rax: Option<u64>,
    table: Option<SourceCrtTableCompletion>,
    cleanup_return_rax: Option<u64>,
}
impl SourceCrtStartupPrefix {
    pub fn source_entry_ecx(&self) -> u32 { self.source_entry_ecx }
    pub fn original_mode_flag(&self) -> Option<u8> { self.original_mode_flag }
    pub fn stored_mode_flag(&self) -> Option<u8> { self.stored_mode_flag }
    pub fn isa_return_rax(&self) -> Option<u64> { self.isa_return_rax }
    pub fn resources_return_rax(&self) -> Option<u64> { self.resources_return_rax }
    pub fn table(&self) -> Option<&SourceCrtTableCompletion> { self.table.as_ref() }
    pub fn cleanup_return_rax(&self) -> Option<u64> { self.cleanup_return_rax }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtStartupCompletion {
    prefix: SourceCrtStartupPrefix,
    return_al: u8,
}
impl SourceCrtStartupCompletion {
    pub fn prefix(&self) -> &SourceCrtStartupPrefix { &self.prefix }
    pub fn return_al(&self) -> u8 { self.return_al }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtStartupProgress {
    Request(SourceCrtStartupRequest),
    Complete(SourceCrtStartupCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtStartupError {
    RelocatedTableRangeRequired { supplied_begin: u64 },
    ReceiptBindingRequired { pending: SourceCrtStartupProgress, actual: SourceCrtStartupReceipt },
    ActualNonvolatileContinuationRequired { pending: SourceCrtStartupRequest, actual: SourceCrtStartupReceipt },
}
impl SourceCrtStartupError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::RelocatedTableRangeRequired { .. } => "55de6c requires a nonoverflowing actual relocated initializer table range",
            Self::ReceiptBindingRequired { .. } => "55de6c actual completion does not match pending source call/read/store/table binding",
            Self::ActualNonvolatileContinuationRequired { .. } => "55de6c actual callee return changed saved source RBX1",
        }
    }
}
pub struct SourceCrtStartupServiceFailure<E> {
    pub cause: E,
    pub pending: SourceCrtStartupRequest,
}
pub struct SourceCrtStartupRejectedFailure<E> {
    pub cause: E,
    pub completion: SourceCrtStartupCompletion,
}

pub struct SourceCrtStartup55de6c {
    prefix: SourceCrtStartupPrefix,
    actual_table_begin: u64,
    actual_table_end: u64,
    progress: SourceCrtStartupProgress,
    failed_service: bool,
}
impl SourceCrtStartup55de6c {
    pub fn begin_with_explicit_entry(source_entry_ecx: u32, actual_table_begin: u64)
        -> Result<Self, SourceCrtStartupError>
    {
        let Some(actual_table_end) = actual_table_begin.checked_add(0x100) else {
            return Err(SourceCrtStartupError::RelocatedTableRangeRequired { supplied_begin: actual_table_begin });
        };
        Ok(Self {
            prefix: SourceCrtStartupPrefix { source_entry_ecx, original_mode_flag: None, stored_mode_flag: None,
                isa_return_rax: None, resources_return_rax: None, table: None, cleanup_return_rax: None },
            actual_table_begin, actual_table_end, failed_service: false,
            progress: SourceCrtStartupProgress::Request(SourceCrtStartupRequest::ReadModeFlag85f430),
        })
    }
    pub fn progress(&self) -> SourceCrtStartupProgress { self.progress }

    pub fn prefix(&self) -> &SourceCrtStartupPrefix { &self.prefix }
    pub fn request(&self) -> Option<SourceCrtStartupRequest> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceCrtStartupRequest> {
        match self.progress { SourceCrtStartupProgress::Request(request) => Some(request), _ => None }
    }
    pub fn accept_actual_completion(&mut self, actual: SourceCrtStartupReceipt)
        -> Result<SourceCrtStartupProgress, SourceCrtStartupError>
    {
        self.failed_service = true;
        match (self.progress, actual) {
            (SourceCrtStartupProgress::Request(SourceCrtStartupRequest::ReadModeFlag85f430),
                SourceCrtStartupReceipt::ModeFlag { value }) => {
                self.prefix.original_mode_flag = Some(value);
                let selected = if self.prefix.source_entry_ecx == 0 { 1 } else { value };
                self.next(SourceCrtStartupRequest::StoreModeFlag85f430 { value: selected });
            }
            (SourceCrtStartupProgress::Request(SourceCrtStartupRequest::StoreModeFlag85f430 { value: expected }),
                SourceCrtStartupReceipt::ModeFlagStored { value }) if value == expected => {
                self.prefix.stored_mode_flag = Some(value);
                self.next(SourceCrtStartupRequest::Call(SourceCrtStartupCall::GeneralIsa561dd4));
            }
            (SourceCrtStartupProgress::Request(SourceCrtStartupRequest::Call(expected)),
                SourceCrtStartupReceipt::CallReturned { call, actual_rax, actual_rbx }) if call == expected => {
                if actual_rbx != 1 {
                    return Err(SourceCrtStartupError::ActualNonvolatileContinuationRequired {
                        pending: SourceCrtStartupRequest::Call(expected), actual });
                }
                match expected {
                    SourceCrtStartupCall::GeneralIsa561dd4 => {
                        self.prefix.isa_return_rax = Some(actual_rax);
                        self.next(SourceCrtStartupRequest::Call(SourceCrtStartupCall::Resources564ccc));
                    }
                    SourceCrtStartupCall::Resources564ccc => {
                        self.prefix.resources_return_rax = Some(actual_rax);
                        if actual_rax as u8 == 0 { self.complete(0); }
                        else { self.next(SourceCrtStartupRequest::InitializerTable {
                            actual_begin: self.actual_table_begin, actual_end: self.actual_table_end }); }
                    }
                    SourceCrtStartupCall::FailureCleanup564cf4 => {
                        self.prefix.cleanup_return_rax = Some(actual_rax);
                        self.complete(0);
                    }
                }
            }
            (SourceCrtStartupProgress::Request(SourceCrtStartupRequest::InitializerTable { actual_begin, actual_end }),
                SourceCrtStartupReceipt::InitializerTableReturned { completion, actual_rbx })
                if completion.actual_begin() == actual_begin && completion.actual_end() == actual_end => {
                if actual_rbx != 1 {
                    return Err(SourceCrtStartupError::ActualNonvolatileContinuationRequired {
                        pending: SourceCrtStartupRequest::InitializerTable { actual_begin, actual_end }, actual });
                }
                self.prefix.table = Some(completion);
                if completion.return_al() != 0 { self.complete(1); }
                else { self.next(SourceCrtStartupRequest::Call(SourceCrtStartupCall::FailureCleanup564cf4)); }
            }
            _ => return Err(SourceCrtStartupError::ReceiptBindingRequired { pending: self.progress, actual }),
        }
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E)
        -> Result<SourceCrtStartupServiceFailure<E>, SourceCrtStartupRejectedFailure<E>>
    {
        match self.progress {
            SourceCrtStartupProgress::Request(pending) => {
                self.failed_service = true;
                Ok(SourceCrtStartupServiceFailure { cause, pending })
            }
            SourceCrtStartupProgress::Complete(completion) => Err(SourceCrtStartupRejectedFailure { cause, completion }),
        }
    }
    fn next(&mut self, request: SourceCrtStartupRequest) { self.progress = SourceCrtStartupProgress::Request(request); }
    fn complete(&mut self, return_al: u8) {
        self.progress = SourceCrtStartupProgress::Complete(SourceCrtStartupCompletion { prefix: self.prefix, return_al });
    }
}
