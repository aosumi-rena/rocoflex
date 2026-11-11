pub(super) fn equal(left: &str, right: &str) -> bool {
    left.split('\0').next() == right.split('\0').next()
}
