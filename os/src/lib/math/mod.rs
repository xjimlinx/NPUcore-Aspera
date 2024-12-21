pub fn is_power_of(num: u64, base: u64) -> bool {
    if num == 1 {
        return true;
    }
    if base <= 1 || num < base {
        return false;
    }
    if num % base != 0 {
        return false;
    }
    is_power_of(num / base, base)
}
