use super::Direction;
use rlua::Lua as Rlua;
use std::sync::{Arc, Mutex};
use tokio::prelude::*;
use tokio::sync::mpsc;

pub struct Lua {
	lua: rlua::Lua,
	connection_data: Arc<Mutex<super::ConnectionData>>,
}

impl Lua {
	pub fn new(
		connection_data: Arc<Mutex<super::ConnectionData>>,
		write_sockets: mpsc::Sender<Direction<Vec<u8>>>,
	) -> Lua {
		let lua = Lua {
			lua: Rlua::new(),
			connection_data: connection_data.clone(),
		};
		lua.set_registers();
		lua.set_default_funcs();
		lua.set_read_func();
		lua.set_write_func(write_sockets);
		lua
	}
	fn set_registers(&self) {
		self.lua.context(|lua_ctx| {
			let globals = lua_ctx.globals();
			globals.set("incomming", -1).unwrap();
			globals.set("outgoing", 1).unwrap();
		});
	}
	pub fn run(self) -> std::thread::JoinHandle<()> {
		std::thread::Builder::new()
			.name("lua_thread".to_string())
			.spawn(move || {
				self.lua.context(|lua_ctx| {
					let script = std::fs::read_to_string("script.lua").unwrap();
					lua_ctx.load(&script).exec().unwrap();
				});
				let mut cd = self.connection_data.lock().unwrap();
				cd.push_modified(Direction::None);
			})
			.unwrap()
	}
	fn set_default_funcs(&self) {
		self.lua.context(|lua_ctx| {
			lua_ctx
				.load(
					r#"
						function data2str(data)
    						local str = ""
						    for k, v in pairs(data) do
        						str = str .. string.char(v)
						    end
    						return str
						end

						function str2data(str)
    					local table = {}
					    for i = 1, #str do
        					table[i] = string.byte(str:sub(i, i))
					    end
    					return table
					end
			"#,
				)
				.exec()
				.unwrap();
		});
	}
	fn set_read_func(&self) {
		let connection_data = self.connection_data.clone();
		self.lua.context(|lua_ctx| {
			let recive_data = lua_ctx
				.create_function(move |ctx, buffer_size: usize| loop {
					let mut connection_data = connection_data.lock().unwrap();
					match connection_data.get(buffer_size) {
						Direction::Out(data) => {
							return {
								let table = ctx.create_table().unwrap();
								table.set("status", 1).unwrap();
								table.set("data", data).unwrap();
								table.set("no", connection_data.get_no()).unwrap();
								Ok(table)
							};
						}

						Direction::In(data) => {
							return {
								let table = ctx.create_table().unwrap();
								table.set("status", -1).unwrap();
								table.set("data", data).unwrap();
								table.set("no", connection_data.get_no()).unwrap();
								Ok(table)
							};
						}
						Direction::NotReady => {
							std::mem::drop(connection_data);
							std::thread::park();
							continue;
						}
						Direction::None => {
							return {
								let table = ctx.create_table().unwrap();
								table.set("status", rlua::Value::Nil).unwrap();
								Ok(table)
							};
						}
					}
				})
				.unwrap();
			lua_ctx.globals().set("recive", recive_data).unwrap();
		})
	}
	fn set_write_func(&self, write_sockets: mpsc::Sender<Direction<Vec<u8>>>) {
		let connection_data = self.connection_data.clone();
		self.lua.context(|lua_ctx| {
			let send_data = lua_ctx
				.create_function(move |_, (direction, stri): (i8, Vec<u8>)| {
					let mut cd = connection_data.lock().unwrap();
					let sockets = write_sockets.clone();
					if direction == -1 {
						cd.push_modified(Direction::In(stri.clone()));
						sockets.send(Direction::In(stri.to_vec())).wait().unwrap();
					} else if direction == 1 {
						sockets.send(Direction::Out(stri.to_vec())).wait().unwrap();
						cd.push_modified(Direction::Out(stri.clone()));
					}
					Ok(())
				})
				.unwrap();
			lua_ctx.globals().set("send", send_data).unwrap();
		})
	}
}
