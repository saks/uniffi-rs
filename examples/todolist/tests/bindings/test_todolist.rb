# frozen_string_literal: true

require 'todolist'

include Todolist

def assert(condition)
  raise 'Assertion failed!' unless condition
end

todo = TodoList.new
entry = TodoEntry.new 'Write bindings for strings in records'

todo.add_item('Write ruby bindings')

assert(todo.get_last == 'Write ruby bindings')

todo.add_item('Write tests for bindings')

assert(todo.get_last == 'Write tests for bindings')

todo.add_entry(entry)

assert(todo.get_last == 'Write bindings for strings in records')
assert(todo.get_last_entry.text == 'Write bindings for strings in records')

todo.add_item("Test Ünicode hàndling without an entry can't believe I didn't test this at first 🤣")
assert(todo.get_last == "Test Ünicode hàndling without an entry can't believe I didn't test this at first 🤣")

entry2 = TodoEntry.new("Test Ünicode hàndling in an entry can't believe I didn't test this at first 🤣")
todo.add_entry(entry2)
assert(todo.get_last_entry.text == "Test Ünicode hàndling in an entry can't believe I didn't test this at first 🤣")
