mod protocol_id_test;
mod submessage_flag_test;

mod tests {
    pub fn remove_cdr_header(data: &Vec<u8>) -> Vec<u8> {
        data.iter().skip(4).map(|&x| {x}).collect()
    }
}
