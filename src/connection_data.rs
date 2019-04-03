use super::Direction;
#[derive(Debug)]
pub struct ConnectionData {
    original_data: Vec<Direction<Vec<u8>>>,
    modified_data: Vec<Direction<Vec<u8>>>,
    original_last_position: (usize, usize),
    read_position: (usize, usize),
    modified_last_position: (usize, usize),
    write_position: (usize, usize),
}

impl ConnectionData {
    pub fn new() -> ConnectionData {
        ConnectionData {
            original_data: Vec::new(),
            modified_data: Vec::new(),
            original_last_position: (0, 0),
            read_position: (0, 0),
            modified_last_position: (0, 0),
            write_position: (0, 0),
        }
    }
    pub fn push_modified(&mut self, value: Direction<Vec<u8>>) {
        match self.modified_data.last_mut() {
            Some(Direction::In(last_data)) => match value {
                Direction::In(mut data) => {
                    self.modified_last_position.1 += data.len();
                    last_data.append(&mut data)
                }
                Direction::Out(data) => {
                    self.modified_last_position.0 += 1;
                    self.modified_last_position.1 = data.len();
                    self.modified_data.push(Direction::Out(data))
                }
                Direction::None => {
                    self.modified_last_position.0 += 1;
                    self.modified_last_position.1 = 0;
                    self.modified_data.push(Direction::None)
                }
                _ => unimplemented!(),
            },
            Some(Direction::Out(last_data)) => match value {
                Direction::In(data) => {
                    self.modified_last_position.0 += 1;
                    self.modified_last_position.1 = data.len();
                    self.modified_data.push(Direction::In(data))
                }
                Direction::Out(mut data) => {
                    self.modified_last_position.1 += data.len();
                    last_data.append(&mut data)
                }
                Direction::None => {
                    self.modified_last_position.0 += 1;
                    self.modified_last_position.1 = 0;
                    self.modified_data.push(Direction::None)
                }
                _ => unimplemented!(),
            },
            Some(_) => unimplemented!(),
            None => {
                self.modified_data.push(value);
                self.modified_last_position.1 = self.modified_data.len();
            }
        };
    }
    pub fn push_original(&mut self, value: Direction<Vec<u8>>) {
        match self.original_data.last_mut() {
            Some(Direction::In(last_data)) => match value {
                Direction::In(mut data) => {
                    self.original_last_position.1 += data.len();
                    last_data.append(&mut data)
                }
                Direction::Out(data) => {
                    self.original_last_position.0 += 1;
                    self.original_last_position.1 = data.len();
                    self.original_data.push(Direction::Out(data))
                }
                Direction::None => {
                    self.original_last_position.0 += 1;
                    self.original_last_position.1 = 0;
                    self.original_data.push(Direction::None)
                }
                _ => unimplemented!(),
            },
            Some(Direction::Out(last_data)) => match value {
                Direction::In(data) => {
                    self.original_last_position.0 += 1;
                    self.original_last_position.1 = data.len();
                    self.original_data.push(Direction::In(data))
                }
                Direction::Out(mut data) => {
                    self.original_last_position.1 += data.len();
                    last_data.append(&mut data)
                }
                Direction::None => {
                    self.original_last_position.0 += 1;
                    self.original_last_position.1 = 0;
                    self.original_data.push(Direction::None)
                }
                _ => unimplemented!(),
            },
            Some(_) => unimplemented!(),
            None => {
                self.original_data.push(value);
                self.original_last_position.1 = self.original_data.len();
            }
        };
    }
    pub fn get(&mut self, size: usize) -> Direction<Vec<u8>> {
        loop {
            match self.original_data.get(self.read_position.0) {
                Some(Direction::In(data)) => {
                    if data.len() == self.read_position.1 {
                        match self.original_data.get(self.read_position.0 + 1) {
                            Some(_) => {
                                self.read_position.0 += 1;
                                self.read_position.1 = 0;
                                continue;
                            }
                            None => return Direction::NotReady,
                        }
                    } else if (data.len() - self.read_position.1) <= size {
                        let chunk = data[self.read_position.1..data.len()].to_vec();
                        self.read_position.1 += chunk.len();
                        return Direction::In(chunk);
                    } else {
                        let chunk =
                            data[self.read_position.1..(self.read_position.1 + size)].to_vec();
                        self.read_position.1 += chunk.len();
                        return Direction::In(chunk);
                    }
                }
                Some(Direction::Out(data)) => {
                    if data.len() == self.read_position.1 {
                        match self.original_data.get(self.read_position.0 + 1) {
                            Some(_) => {
                                self.read_position.0 += 1;
                                self.read_position.1 = 0;
                                continue;
                            }
                            None => return Direction::NotReady,
                        }
                    } else if (data.len() - self.read_position.1) <= size {
                        let chunk = data[self.read_position.1..data.len()].to_vec();
                        self.read_position.1 += chunk.len();
                        return Direction::Out(chunk);
                    } else {
                        let chunk =
                            data[self.read_position.1..(self.read_position.1 + size)].to_vec();
                        self.read_position.1 += chunk.len();
                        return Direction::Out(chunk);
                    }
                }
                Some(Direction::None) => {
                    //                    self.read_position.0 += 1;
                    self.read_position.1 = 0;
                    return Direction::None;
                }
                _ => return Direction::NotReady,
            }
        }
    }
    pub fn get_no(&mut self) -> usize {
        self.read_position.0
    } 
    pub fn _get_modified(&mut self, size: usize) -> Direction<Vec<u8>> {
        loop {
            match self.modified_data.get(self.write_position.0) {
                Some(Direction::In(data)) => {
                    if data.len() == self.write_position.1 {
                        match self.modified_data.get(self.write_position.0 + 1) {
                            Some(_) => {
                                self.write_position.0 += 1;
                                self.write_position.1 = 0;
                                continue;
                            }
                            None => return Direction::NotReady,
                        }
                    } else if (data.len() - self.write_position.1) <= size {
                        let chunk = data[self.write_position.1..data.len()].to_vec();
                        self.write_position.1 += chunk.len();
                        return Direction::In(chunk);
                    } else {
                        let chunk =
                            data[self.write_position.1..(self.write_position.1 + size)].to_vec();
                        self.write_position.1 += chunk.len();
                        return Direction::In(chunk);
                    }
                }
                Some(Direction::Out(data)) => {
                    if data.len() == self.write_position.1 {
                        match self.modified_data.get(self.write_position.0 + 1) {
                            Some(_) => {
                                self.write_position.0 += 1;
                                self.write_position.1 = 0;
                                continue;
                            }
                            None => return Direction::NotReady,
                        }
                    } else if (data.len() - self.write_position.1) <= size {
                        let chunk = data[self.write_position.1..data.len()].to_vec();
                        self.write_position.1 += chunk.len();
                        return Direction::Out(chunk);
                    } else {
                        let chunk =
                            data[self.write_position.1..(self.write_position.1 + size)].to_vec();
                        self.write_position.1 += chunk.len();
                        return Direction::Out(chunk);
                    }
                }
                Some(Direction::None) => {
                    //                    self.write_position.0 += 1;
                    self.write_position.1 = 0;
                    return Direction::None;
                }
                _ => return Direction::NotReady,
            }
        }
    }
    // fn request_number(&self) -> usize {
    //     self.read_position.0
    // }
}
