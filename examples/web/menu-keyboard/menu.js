// Shared by the standalone LTR and RTL samples.
(() => {
    const bar = '[role="menubar"]';
    const menuItem = 'button[role="menuitem"]';
    const items = menu => [...menu.querySelectorAll(':scope > ' + menuItem)];
    const openChild = menu => menu.querySelector(':scope > dialog:popover-open');
    const launchers = new Map();
    const returning = new Set();
    for (const button of document.querySelectorAll('button[command="show-popover"]')) {
        const panel = button.commandForElement;
        launchers.set(panel, button);
        panel.addEventListener('beforetoggle', event => {
            returning.delete(panel);
            if (event.newState === 'closed' && panel.contains(document.activeElement)) returning.add(panel);
        });
        panel.addEventListener('toggle', () => {
            const open = panel.matches(':popover-open');
            button.setAttribute('aria-expanded', String(open));
            // Some nested native dismissals leave focus on the body. A newer
            // focus move, or closing an ancestor, takes precedence over repair.
            if (!open && returning.delete(panel) && (document.activeElement === document.body || panel.contains(document.activeElement))
                && button.isConnected && !button.closest('[popover]:not(:popover-open)')) button.focus();
        });
    }

    // Only the most recently focused root item is in its menubar's tab order.
    document.addEventListener('focusin', ({ target }) => {
        for (const panel of returning) {
            if (target !== launchers.get(panel) && !panel.contains(target)) returning.delete(panel);
        }
        if (!target.matches(menuItem) || !target.parentElement.matches(bar)) return;
        for (const item of items(target.parentElement)) item.tabIndex = item === target ? 0 : -1;
    });

    function adjacent(menu, button, step) {
        const siblings = items(menu);
        return siblings[(siblings.indexOf(button) + step + siblings.length) % siblings.length];
    }

    function enter(button, destination) {
        const panel = button.commandForElement;
        if (!panel || button.getAttribute('aria-disabled') === 'true') return;
        if (!panel.matches(':popover-open')) {
            button.focus();
            panel.showPopover({ source: button });
        }
        if (!panel.matches(':popover-open')) return;
        if (destination === 'launcher') button.focus();
        else items(panel).at(destination === 'last' ? -1 : 0)?.focus();
    }

    function changeBranch(root, sibling) {
        const previous = openChild(root);
        if (previous && previous !== sibling.commandForElement) previous.hidePopover();
        sibling.focus();
        if (sibling.commandForElement) enter(sibling, 'launcher');
    }

    // Walk the invokers so a nested leaf can reach its root branch or menu button.
    function rootLauncher(button) {
        let launcher = button;
        while (launchers.has(launcher.parentElement)) launcher = launchers.get(launcher.parentElement);
        return launcher;
    }

    function branchPanel(branch) {
        return branch.parentElement.matches(bar) ? openChild(branch.parentElement) : branch.commandForElement;
    }

    function arrow(button, key) {
        const level = button.parentElement;
        const horizontal = level.getAttribute('aria-orientation') === 'horizontal';
        const rtl = getComputedStyle(level).direction === 'rtl';
        const forward = rtl ? 'ArrowLeft' : 'ArrowRight';
        const back = rtl ? 'ArrowRight' : 'ArrowLeft';
        const next = horizontal ? forward : 'ArrowDown';
        const previous = horizontal ? back : 'ArrowUp';
        const root = level.matches(bar);
        if (key === next || key === previous) {
            const sibling = adjacent(level, button, key === next ? 1 : -1);
            if (root && openChild(level)) changeBranch(level, sibling);
            else sibling.focus();
            return;
        }

        const towardChild = key === (horizontal ? 'ArrowDown' : forward);
        if (root) {
            if (button.commandForElement) enter(button, towardChild ? 'first' : 'last');
            else {
                openChild(level)?.hidePopover();
                adjacent(level, button, towardChild ? 1 : -1).focus();
            }
        } else if (towardChild && button.commandForElement) {
            enter(button, 'first');
        } else {
            const launcher = launchers.get(level);
            if (!towardChild && launchers.has(launcher.parentElement)) {
                level.hidePopover();
                launcher.focus();
                return;
            }
            const branch = rootLauncher(button);
            if (branch.parentElement.matches(bar)) {
                changeBranch(branch.parentElement, adjacent(branch.parentElement, branch, towardChild ? 1 : -1));
            }
        }
    }

    document.addEventListener('keydown', event => {
        if (event.defaultPrevented || event.isComposing || event.altKey || event.metaKey || event.ctrlKey) return;
        const button = event.target;
        if (!button.matches(menuItem + ', button[command="show-popover"]')) return;
        const item = button.matches(menuItem);

        if (event.key === 'Tab' && item) {
            const branch = rootLauncher(button);
            const panel = branchPanel(branch);
            if (panel?.matches(':popover-open')) {
                panel.hidePopover();
                branch.focus(); // Browser continues Tab or Shift+Tab from the logical entry point.
            }
            return;
        }
        if (event.shiftKey) return;
        if (event.key === 'Enter' || event.key === ' ') {
            if (event.repeat || button.getAttribute('aria-disabled') === 'true') event.preventDefault();
            return; // The browser generates the enabled button's click.
        }
        if (!['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(event.key)) return;
        if (item) {
            event.preventDefault();
            arrow(button, event.key);
        } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            event.preventDefault();
            enter(button, event.key === 'ArrowDown' ? 'first' : 'last');
        }
    });

    document.addEventListener('click', event => {
        const button = event.target.closest('button');
        if (!button || event.defaultPrevented) return;
        if (button.getAttribute('aria-disabled') === 'true') {
            event.preventDefault();
            return;
        }
        const panel = button.commandForElement;
        if (panel) {
            // A branch preview can already be visible with focus still on its launcher.
            if (panel.matches(':popover-open')) items(panel)[0]?.focus();
            return; // Native command + autofocus handle a newly opened panel.
        }
        if (button.matches(menuItem)) {
            document.querySelector('output').value = button.textContent;
            branchPanel(rootLauncher(button))?.hidePopover();
        }
    });
})();
