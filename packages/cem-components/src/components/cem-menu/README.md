## Menu and Sub-menu Implementation Checklist

**Context**
Technical overview of web navigation patterns, focusing on ARIA roles, structural requirements, and
accessibility-compliant keyboard interactions.

**Diagnostics**
An automated scan for existing navigation structures on the current page returned the following results:

| Feature                                      | Results Found |
|:---------------------------------------------|:--------------|
| `<nav>` Elements                             | 0             |
| `role="menubar"` or `role="menu"`            | 0             |
| Common Sub-menu Selectors (`.submenu`, etc.) | 0             |

**Implementation Methods**
The following patterns were identified as the industry standards for menu development:

* **Disclosure Pattern (Recommended):** Utilizes a `<button>` with `aria-expanded` and `aria-controls` attributes to
  toggle a `<ul>` list. This is the preferred method for standard responsive website navigation.
* **Navigation Menubar Pattern:** Employs `role="menubar"` and `role="menuitem"`. This targets complex,
  application-style interfaces where users expect desktop-like navigation.

**Keyboard Interaction Standards**
Keyboard behavior must be explicitly managed to ensure accessibility:

| Key(s)            | Disclosure Pattern              | Menubar Pattern                |
|:------------------|:--------------------------------|:-------------------------------|
| **Tab**           | Move focus to the next trigger. | Exit the entire menubar.       |
| **Enter / Space** | Toggle sub-menu visibility.     | Activate menu item.            |
| **Arrow Keys**    | N/A                             | Navigate between items/levels. |
| **Escape**        | Close sub-menu; return focus.   | Close sub-menu.                |

**Technical Guidance**
The following logic and styling represent a potential implementation for a robust sub-menu system:

`````css
/* Ensure sub-menus are hidden from both sight and assistive technology */
.sub-menu {
    display: none;
    position: absolute;
}

/* Use aria-expanded state to toggle visibility */
.menu-trigger[aria-expanded="true"] + .sub-menu {
    display: block;
}
`````

`````js
/* Illustrative toggle logic for the Disclosure pattern */
const trigger = document.querySelector('.menu-trigger');
trigger.addEventListener('click', () => {
    const isExpanded = trigger.getAttribute('aria-expanded') === 'true';
    trigger.setAttribute('aria-expanded', !isExpanded);
});
`````

**Actionable Recommendations**

* **Visibility:** Use `display: none` or `visibility: hidden` for collapsed menus. Avoid using `opacity: 0` alone, as
  the elements remain in the tab order.
* **Positioning:** Apply `position: absolute` to sub-menus relative to a `position: relative` parent container to avoid
  breaking document flow.
* **Focus Management:** For the Menubar pattern, use `tabindex="-1"` on non-active items and manage the "roving
  tabindex" via JavaScript arrow key listeners.

*Note: The code fixes and findings above were identified on a live page in DevTools. When applying them to your
codebase, please adapt them to your project's specific technical stack (e.g., Tailwind CSS classes, CSS modules,
framework components) rather than applying them as literal CSS overrides.*
