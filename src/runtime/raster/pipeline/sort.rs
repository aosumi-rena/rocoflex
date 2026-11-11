pub type Record = [u32; 4];
#[derive(Debug)]
pub enum Error { NonFiniteKey, Size }
trait Keyed: Copy { type Key: PartialOrd; fn source_key(self) -> Self::Key; }
impl Keyed for Record { type Key = f32; fn source_key(self) -> f32 { f32::from_bits(self[0]) } }
impl Keyed for [i32; 2] { type Key = i32; fn source_key(self) -> i32 { self[0] } }
fn key<T: Keyed>(r: T) -> T::Key { r.source_key() }

fn median3<T: Keyed>(a: &mut [T], first: usize, middle: usize, last: usize) {
    if key(a[first]) > key(a[middle]) { a.swap(first, middle); }
    if key(a[middle]) > key(a[last]) {
        a.swap(middle, last);
        if key(a[first]) > key(a[middle]) { a.swap(first, middle); }
    }
}
fn guess<T: Keyed>(a: &mut [T], first: usize, last: usize) -> usize {
    let n = last - first;
    let middle = first + n / 2;
    let tail = last - 1;

    if n >= 42 {
        let step = n / 8;
        median3(a, first, first + step, first + 2 * step);
        median3(a, middle - step, middle, middle + step);
        median3(a, tail - 2 * step, tail - step, tail);
        median3(a, first + step, middle, tail - step);
    } else { median3(a, first, middle, tail); }
    middle
}

fn partition<T: Keyed>(a: &mut [T], first: usize, last: usize) -> (usize, usize) {
    let middle = guess(a, first, last);
    let mut equal_first = middle;
    let mut equal_last = middle + 1;
    while equal_first > first && key(a[equal_first - 1]) == key(a[equal_first]) {
        equal_first -= 1;
    }
    while equal_last < last && key(a[equal_last]) == key(a[equal_first]) {
        equal_last += 1;
    }
    let mut left = equal_first;
    let mut right = equal_last;
    loop {
        while right < last {
            if key(a[right]) > key(a[equal_first]) { right += 1; }
            else if key(a[equal_first]) > key(a[right]) { break; }
            else {
                if right != equal_last { a.swap(equal_last, right); }
                equal_last += 1;
                right += 1;
            }
        }
        while left > first {
            let previous = left - 1;
            if key(a[equal_first]) > key(a[previous]) { left = previous; }
            else if key(a[previous]) > key(a[equal_first]) { break; }
            else {
                let new_first = equal_first - 1;
                if left != equal_first { a.swap(new_first, previous); }
                equal_first = new_first;
                left = previous;
            }
        }
        if left == first && right == last { return (equal_first, equal_last); }
        if left == first {

            if equal_last != right { a.swap(equal_first, equal_last); }
            a.swap(equal_first, right);
            equal_first += 1;
            equal_last += 1;
            right += 1;
        } else if right == last {

            left -= 1;
            equal_first -= 1;
            if left != equal_first { a.swap(left, equal_first); }
            equal_last -= 1;
            a.swap(equal_first, equal_last);
        } else {
            left -= 1;
            a.swap(right, left);
            right += 1;
        }
    }
}

fn adjust_heap<T: Keyed>(a: &mut [T], first: usize, n: usize, top: usize, value: T) {
    let mut hole = top;
    let last_parent = (n - 1) / 2;
    while hole < last_parent {
        let left = 2 * hole + 1;
        let right = left + 1;
        let child = if key(a[first + left]) > key(a[first + right]) { left } else { right };
        a[first + hole] = a[first + child];
        hole = child;
    }
    if n % 2 == 0 && hole == last_parent {
        a[first + hole] = a[first + n - 1];
        hole = n - 1;
    }
    while hole > top {
        let parent = (hole - 1) / 2;
        if !(key(value) > key(a[first + parent])) { break; }
        a[first + hole] = a[first + parent];
        hole = parent;
    }
    a[first + hole] = value;
}
fn heap<T: Keyed>(a: &mut [T], first: usize, last: usize) {
    let n = last - first;
    for top in (0..n / 2).rev() {
        let value = a[first + top];
        adjust_heap(a, first, n, top, value);
    }
    for end in (1..n).rev() {
        let value = a[first + end];
        a[first + end] = a[first];
        adjust_heap(a, first, end, 0, value);
    }
}
fn insertion<T: Keyed>(a: &mut [T], first: usize, last: usize) {
    for next in first + 1..last {
        let value = a[next];
        let mut hole = next;
        while hole > first && key(a[hole - 1]) > key(value) {
            a[hole] = a[hole - 1];
            hole -= 1;
        }
        a[hole] = value;
    }
}
fn sort_range<T: Keyed>(a: &mut [T], mut first: usize, mut last: usize, mut budget: usize) {
    while last - first >= 33 {
        if budget == 0 { heap(a, first, last); return; }
        let (equal_first, equal_last) = partition(a, first, last);
        budget = (budget >> 1) + (budget >> 2);
        if equal_first - first >= last - equal_last {
            sort_range(a, equal_last, last, budget);
            last = equal_first;
        } else {
            sort_range(a, first, equal_first, budget);
            first = equal_last;
        }
    }
    if first < last { insertion(a, first, last); }
}
pub fn sort_source(a: &mut [Record]) -> Result<(), Error> {
    if a.len() > i64::MAX as usize / 16 { return Err(Error::Size); }
    if a.iter().any(|r| !key(*r).is_finite()) { return Err(Error::NonFiniteKey); }
    let n = a.len();
    sort_range(a, 0, n, n);
    Ok(())
}

pub fn sort_integer_pairs_source(a: &mut [[i32; 2]]) -> Result<(), Error> {
    if a.len() > i64::MAX as usize / 8 { return Err(Error::Size); }
    let n = a.len();
    sort_range(a, 0, n, n);
    Ok(())
}
