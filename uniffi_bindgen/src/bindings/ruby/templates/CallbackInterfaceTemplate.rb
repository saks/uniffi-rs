{%- let cbi = ci.get_callback_interface_definition(name).unwrap() %}
{%- let cbi_name = name %}

# The FfiConverter for the {{ name }} callback interface.
CallbackInterface{{ name|class_name_rb }}FfiConverter = CallbackInterfaceFfiConverter.new

{% include "CallbackInterfaceImpl.rb" %}
