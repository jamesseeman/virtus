# Virtus

A virtual machine orchestrator built on libvirt, written in Rust.

The compute units in virtus are *servers* (synonymous with virtual machine), not instances.

### To-do

**OVS:**
- replace Object trait w Record enum for OVS
- parameterize interface/port bridge
- simplify (de-abstract) ovs api
- check if bridge/port exists before creating

I plan on adding support for containers, but at the moment that is not the focus. There are plenty of orchestration tools out there for containers, but I have not found that to be the case for virtual machines. 
