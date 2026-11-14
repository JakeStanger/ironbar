---
name: New module request
about: Suggest a module that does not exist in Ironbar yet
title: ''
type: New Module
assignees: ''
---

**What should the module show or do?**
<!-- A clear description of the module and the problem it solves. -->

**Data source**
<!--
Where would the information come from? For example a Wayland protocol, a D-Bus service, a compositor IPC, a file in `/sys` or `/proc`, or a CLI tool. The hard part is usually getting a working client, so any pointers here help a lot.

In general the aim is to rely on native clients that provide event-driven systems. 
Compositor-specific implementations are used where there is no other option.
Calling out to external processes is not accepted.
-->

**Compositor / environment specifics**
<!-- Does this only make sense on certain compositors? If so, list them. -->

**Can a custom module do this already?**
<!-- 
Provide any details on if this can already be acomplished using one of the custom modules,
or via the Lua API. Detail why this approach is not suitable

If it's not possible, detail why not.
-->