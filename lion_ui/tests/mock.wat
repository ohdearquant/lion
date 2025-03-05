(module
  (func $add (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
  
  (func $subtract (export "subtract") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.sub)
    
  (func $multiply (export "multiply") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.mul)
)
