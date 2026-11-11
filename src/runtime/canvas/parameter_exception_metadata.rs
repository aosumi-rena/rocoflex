#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Member {
    pub offset: u32, pub inline_slot: i32, pub inline_first: bool,
    pub parameter_pc: u32, pub member_pc: u32, pub inline_pc: u32,
    pub vtable_pc: u32, pub call_pc: u32, pub reload_pc: u32, pub clear_pc: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action { Member(Member), Direct { frame_slot: i32, read_pc: u32, call_pc: u32, target: u32 } }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    pub state: i32, pub next_state: i32, pub funclet: u32, pub body_pc: u32,
    pub epilogue_pc: u32, pub ret_pc: u32, pub actions: [Option<Action>; 2],
}
const fn member(offset: u32, slot: i32, first: bool, p: u32, m: u32, i: u32, v: u32, call: u32, reload: u32, clear: u32) -> Option<Action> {
    Some(Action::Member(Member { offset, inline_slot: slot, inline_first: first, parameter_pc: p,
        member_pc: m, inline_pc: i, vtable_pc: v, call_pc: call, reload_pc: reload, clear_pc: clear }))
}
pub const PROFILES: [Profile; 8] = [
    Profile { state: 0, next_state: -1, funclet: 0x2b3980, body_pc: 0x2b39a2, epilogue_pc: 0x2b39b4, ret_pc: 0x2b39ca,
        actions: [Some(Action::Direct { frame_slot: 0x40, read_pc: 0x2b39a2, call_pc: 0x2b39a6, target: 0x2b3a50 }),
            Some(Action::Direct { frame_slot: 0x28, read_pc: 0x2b39ab, call_pc: 0x2b39af, target: 0x2b39d0 })] },
    Profile { state: 1, next_state: 0, funclet: 0x2b37b0, body_pc: 0x2b37d2, epilogue_pc: 0x2b37fe, ret_pc: 0x2b3814,
        actions: [member(0xd8, 0x38, false, 0x2b37d2, 0x2b37d6, 0x2b37e2, 0x2b37e9, 0x2b37ec, 0x2b37ef, 0x2b37f3), None] },
    Profile { state: 2, next_state: 1, funclet: 0x2b3710, body_pc: 0x2b3732, epilogue_pc: 0x2b378d, ret_pc: 0x2b37a3,
        actions: [member(0x158, 0x20, true, 0x2b3736, 0x2b373a, 0x2b3732, 0x2b374c, 0x2b374f, 0x2b3752, 0x2b3756),
            member(0x118, 0x30, false, 0x2b3761, 0x2b3765, 0x2b3771, 0x2b3778, 0x2b377b, 0x2b377e, 0x2b3782)] },
    Profile { state: 3, next_state: 1, funclet: 0x2b36a0, body_pc: 0x2b36c2, epilogue_pc: 0x2b36ee, ret_pc: 0x2b3704,
        actions: [member(0x118, 0x30, false, 0x2b36c2, 0x2b36c6, 0x2b36d2, 0x2b36d9, 0x2b36dc, 0x2b36df, 0x2b36e3), None] },
    Profile { state: 4, next_state: 0, funclet: 0x2b3630, body_pc: 0x2b3652, epilogue_pc: 0x2b367e, ret_pc: 0x2b3694,
        actions: [member(0xd8, 0x38, false, 0x2b3652, 0x2b3656, 0x2b3662, 0x2b3669, 0x2b366c, 0x2b366f, 0x2b3673), None] },
    Profile { state: 5, next_state: 0, funclet: 0x2b3930, body_pc: 0x2b3952, epilogue_pc: 0x2b395b, ret_pc: 0x2b3971,
        actions: [Some(Action::Direct { frame_slot: 0x20, read_pc: 0x2b3952, call_pc: 0x2b3956, target: 0x2b39d0 }), None] },
    Profile { state: 6, next_state: 5, funclet: 0x2b3890, body_pc: 0x2b38b2, epilogue_pc: 0x2b390a, ret_pc: 0x2b3920,
        actions: [member(0x1e0, 0x30, false, 0x2b38b2, 0x2b38b6, 0x2b38c2, 0x2b38c9, 0x2b38cc, 0x2b38cf, 0x2b38d3),
            member(0x1a0, 0x38, false, 0x2b38de, 0x2b38e2, 0x2b38ee, 0x2b38f5, 0x2b38f8, 0x2b38fb, 0x2b38ff)] },
    Profile { state: 7, next_state: 5, funclet: 0x2b3820, body_pc: 0x2b3842, epilogue_pc: 0x2b386e, ret_pc: 0x2b3884,
        actions: [member(0x1a0, 0x38, false, 0x2b3842, 0x2b3846, 0x2b3852, 0x2b3859, 0x2b385c, 0x2b385f, 0x2b3863), None] },
];

pub fn state_at_normalized_pc(pc: u32) -> Option<i32> {
    if !(0x2b3000..0x2b3629).contains(&pc) { return None; }
    let entries = [(0x2b3000, -1), (0x2b3132, 4), (0x2b316c, 3), (0x2b31a2, 2),
        (0x2b31fe, 7), (0x2b3238, 6), (0x2b3241, -1)];
    let mut state = -1;
    for (start, value) in entries { if pc < start { break; } state = value; }
    Some(state)
}
