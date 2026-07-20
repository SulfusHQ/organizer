const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let currentRules = [];
let editingRuleId = null;

document.addEventListener("DOMContentLoaded", () => {
    const navLinks = document.querySelectorAll('.nav-links li');
    const views = document.querySelectorAll('.view');
    
    navLinks.forEach(link => {
        link.addEventListener('click', () => {
            navLinks.forEach(l => l.classList.remove('active'));
            views.forEach(v => v.classList.remove('active'));
            link.classList.add('active');
            const targetId = link.getAttribute('data-target');
            document.getElementById(targetId).classList.add('active');
            if (targetId === 'activity-view') loadHistory();
            if (targetId === 'settings-view') loadSettings();
        });
    });

    // Settings Wiring
    document.getElementById('btn-change-folder').addEventListener('click', async () => {
        try {
            const selected = await invoke('pick_folder');
            if (selected) {
                const settings = await invoke('get_settings');
                settings.watch_folder = selected;
                await invoke('update_settings', { newSettings: settings });
                document.getElementById('current-watch-folder').innerText = selected;
                alert('Watch folder updated! Please restart the app for the new folder to take effect.');
            }
        } catch (e) {
            console.error(e);
        }
    });

    document.getElementById('toggle-autostart').addEventListener('change', async (e) => {
        const isChecked = e.target.checked;
        try {
            if (isChecked) {
                await invoke('plugin:autostart|enable');
            } else {
                await invoke('plugin:autostart|disable');
            }
            const settings = await invoke('get_settings');
            settings.autostart = isChecked;
            await invoke('update_settings', { newSettings: settings });
        } catch (err) {
            console.error("Failed to toggle autostart", err);
            e.target.checked = !isChecked; // Revert UI
        }
    });

    const ruleModal = document.getElementById('rule-modal');
    
    document.getElementById('btn-add-rule').addEventListener('click', () => {
        editingRuleId = null;
        document.getElementById('rule-name').value = '';
        document.getElementById('rule-match-logic').value = 'All';
        document.getElementById('conditions-container').innerHTML = '';
        document.getElementById('actions-container').innerHTML = '';
        addConditionRow();
        addActionRow();
        document.querySelector('#modal-title').innerText = 'Create New Rule';
        ruleModal.classList.add('active');
    });

    document.getElementById('btn-cancel-modal').addEventListener('click', () => {
        ruleModal.classList.remove('active');
    });

    document.getElementById('btn-add-condition').addEventListener('click', () => addConditionRow());
    document.getElementById('btn-add-action').addEventListener('click', () => addActionRow());

    document.getElementById('btn-save-rule').addEventListener('click', async () => {
        const name = document.getElementById('rule-name').value || 'Untitled Rule';
        const match_type = document.getElementById('rule-match-logic').value;
        const active = true;

        const conditions = [];
        document.querySelectorAll('.condition-row').forEach(row => {
            const type = row.querySelector('.cond-type').value;
            const op = row.querySelector('.cond-op').value;
            const val = row.querySelector('.cond-val').value;
            
            if(type === 'Size') {
                const unit = parseInt(row.querySelector('.cond-unit').value) || 1;
                const bytes = (parseInt(val) || 0) * unit;
                conditions.push({ type: 'Size', value: { operator: op, bytes: bytes } });
            } else {
                conditions.push({ type, value: { operator: op, text: val } });
            }
        });

        const actions = [];
        document.querySelectorAll('.action-row').forEach(row => {
            const type = row.querySelector('.act-type').value;
            if (type === 'Move') {
                const val = row.querySelector('.act-val').value;
                actions.push({ type: 'Move', value: { target_folder: val } });
            } else if (type === 'Rename') {
                const val = row.querySelector('.act-val').value;
                actions.push({ type: 'Rename', value: { pattern: val } });
            } else if (type === 'Delete') {
                const delayInput = row.querySelector('.act-delay');
                const delay = delayInput ? parseInt(delayInput.value) || 0 : 0;
                actions.push({ type: 'Delete', value: { delay_days: delay } });
            }
        });

        const newRule = {
            id: editingRuleId || `rule_${Date.now()}`,
            name,
            active,
            match_type,
            conditions,
            actions
        };

        if (editingRuleId) {
            const idx = currentRules.findIndex(r => r.id === editingRuleId);
            if(idx !== -1) currentRules[idx] = newRule;
        } else {
            currentRules.push(newRule);
        }

        try {
            await invoke('save_rules', { rules: currentRules });
            ruleModal.classList.remove('active');
            renderRules();
        } catch(e) {
            alert('Failed to save rules: ' + e);
        }
    });

    const deleteModal = document.getElementById('delete-modal');
    document.getElementById('btn-cancel-delete').addEventListener('click', () => {
        deleteModal.classList.remove('active');
        window.ruleToDelete = null;
    });
    
    document.getElementById('btn-confirm-delete').addEventListener('click', async () => {
        if (window.ruleToDelete) {
            currentRules = currentRules.filter(r => r.id !== window.ruleToDelete);
            try {
                await invoke('save_rules', { rules: currentRules });
                renderRules();
            } catch(e) {
                alert("Error deleting rule: " + e);
            }
            deleteModal.classList.remove('active');
            window.ruleToDelete = null;
        }
    });

    loadRules();
    loadSettings();
    listen('history-updated', () => {
        if (document.getElementById('activity-view').classList.contains('active')) loadHistory();
    });
});

async function loadSettings() {
    try {
        const settings = await invoke('get_settings');
        document.getElementById('current-watch-folder').innerText = settings.watch_folder;
        
        try {
            const autostartEnabled = await invoke('plugin:autostart|is_enabled');
            document.getElementById('toggle-autostart').checked = autostartEnabled;
        } catch (e) {
            console.warn("Autostart plugin error", e);
            document.getElementById('toggle-autostart').checked = settings.autostart;
        }
    } catch(e) {
        console.error("Failed to load settings:", e);
    }
}

function addConditionRow(condData = null) {
    const container = document.getElementById('conditions-container');
    const row = document.createElement('div');
    row.className = 'builder-row condition-row';
    
    row.innerHTML = `
        <select class="cond-type">
            <option value="Extension">Extension</option>
            <option value="Name">File Name</option>
            <option value="Size">File Size</option>
        </select>
        <select class="cond-op"></select>
        <div class="cond-val-container" style="flex: 2; display: flex;"></div>
        <button class="btn-remove-row" onclick="this.parentElement.remove()">×</button>
    `;

    const typeSel = row.querySelector('.cond-type');
    const opSel = row.querySelector('.cond-op');
    const valContainer = row.querySelector('.cond-val-container');

    function setupTokenInput(container, datalistId) {
        container.innerHTML = `
            <div class="token-input-wrapper">
                <input type="text" class="token-input-field" list="${datalistId}" placeholder="Type and press Enter...">
            </div>
            <input type="hidden" class="cond-val">
        `;

        const wrapper = container.querySelector('.token-input-wrapper');
        const input = container.querySelector('.token-input-field');
        const hiddenVal = container.querySelector('.cond-val');
        
        let tokens = [];

        function renderTokens() {
            wrapper.querySelectorAll('.token').forEach(t => t.remove());
            tokens.forEach((token, index) => {
                const tDiv = document.createElement('div');
                tDiv.className = 'token';
                tDiv.innerHTML = `${token} <span class="token-remove" data-index="${index}">×</span>`;
                wrapper.insertBefore(tDiv, input);
            });
            wrapper.querySelectorAll('.token-remove').forEach(btn => {
                btn.addEventListener('click', (e) => {
                    const idx = parseInt(e.target.getAttribute('data-index'));
                    tokens.splice(idx, 1);
                    hiddenVal.value = tokens.join(',');
                    renderTokens();
                });
            });
        }

        input.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                const val = input.value.trim();
                if (val && !tokens.includes(val)) {
                    tokens.push(val);
                    input.value = '';
                    hiddenVal.value = tokens.join(',');
                    renderTokens();
                }
            }
        });
        
        input.addEventListener('change', () => {
            const val = input.value.trim();
            if (val && !tokens.includes(val)) {
                tokens.push(val);
                input.value = '';
                hiddenVal.value = tokens.join(',');
                renderTokens();
            }
        });

        return {
            setTokens: (str) => {
                tokens = str ? str.split(',').map(t => t.trim()).filter(t => t) : [];
                hiddenVal.value = tokens.join(',');
                renderTokens();
            }
        };
    }

    function updateRow() {
        const type = typeSel.value;
        if (type === 'Size') {
            opSel.innerHTML = `
                <option value="GreaterThan">Greater Than</option>
                <option value="LessThan">Less Than</option>
                <option value="Equals">Equals</option>
            `;
            valContainer.innerHTML = `
                <input type="number" class="cond-val" placeholder="e.g. 5" style="flex: 1; border-right: none; border-radius: 4px 0 0 4px;">
                <select class="cond-unit" style="width: 70px; flex: none; border-radius: 0 4px 4px 0;">
                    <option value="1">B</option>
                    <option value="1024">KB</option>
                    <option value="1048576" selected>MB</option>
                    <option value="1073741824">GB</option>
                </select>
            `;
        } else if (type === 'Extension') {
            opSel.innerHTML = `
                <option value="Is">Is</option>
                <option value="IsNot">Is Not</option>
                <option value="Contains">Contains</option>
            `;
            const tokenUi = setupTokenInput(valContainer, 'ext-suggestions');
            valContainer.insertAdjacentHTML('beforeend', `
                <datalist id="ext-suggestions">
                    <option value="jpg"><option value="png"><option value="pdf">
                    <option value="docx"><option value="mp4"><option value="zip">
                    <option value="exe"><option value="txt"><option value="csv">
                </datalist>
            `);
            valContainer._tokenUi = tokenUi;
        } else {
            opSel.innerHTML = `
                <option value="Is">Is</option>
                <option value="IsNot">Is Not</option>
                <option value="Contains">Contains</option>
                <option value="StartsWith">Starts With</option>
                <option value="EndsWith">Ends With</option>
            `;
            const tokenUi = setupTokenInput(valContainer, '');
            valContainer._tokenUi = tokenUi;
        }
    }

    typeSel.addEventListener('change', updateRow);
    updateRow();

    if (condData) {
        typeSel.value = condData.type;
        updateRow();
        if(condData.type === 'Size') {
            opSel.value = condData.value.operator;
            let bytes = condData.value.bytes;
            let unit = 1;
            if (bytes > 0 && bytes % 1073741824 === 0) { unit = 1073741824; bytes /= unit; }
            else if (bytes > 0 && bytes % 1048576 === 0) { unit = 1048576; bytes /= unit; }
            else if (bytes > 0 && bytes % 1024 === 0) { unit = 1024; bytes /= unit; }
            
            row.querySelector('.cond-val').value = bytes;
            row.querySelector('.cond-unit').value = unit;
        } else {
            opSel.value = condData.value.operator;
            valContainer._tokenUi.setTokens(condData.value.text);
        }
    }
    container.appendChild(row);
}

function addActionRow(actData = null) {
    const container = document.getElementById('actions-container');
    const row = document.createElement('div');
    row.className = 'builder-row action-row';
    
    row.innerHTML = `
        <select class="act-type">
            <option value="Move">Move to Folder</option>
            <option value="Rename">Rename</option>
            <option value="Delete">Delete</option>
        </select>
        <div class="act-val-container" style="flex: 2; display: flex;"></div>
        <button class="btn-remove-row" onclick="this.parentElement.remove()">×</button>
    `;

    const typeSel = row.querySelector('.act-type');
    const valContainer = row.querySelector('.act-val-container');

    function updateRow() {
        const type = typeSel.value;
        if (type === 'Move') {
            valContainer.innerHTML = `
                <input type="text" class="act-val" placeholder="e.g. C:\\Archive" style="flex: 1; border-right: none; border-radius: 4px 0 0 4px;">
                <button class="btn-secondary btn-pick-folder" title="Browse Folder" style="border-radius: 0 4px 4px 0; padding: 0 12px; border: 1px solid var(--border); border-left: none;">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>
                </button>
            `;
            valContainer.querySelector('.btn-pick-folder').addEventListener('click', async () => {
                try {
                    const selected = await invoke('pick_folder');
                    if (selected) {
                        valContainer.querySelector('.act-val').value = selected;
                    }
                } catch(e) { console.error("Dialog error", e); }
            });
        } else if (type === 'Rename') {
            valContainer.innerHTML = `
                <input type="text" class="act-val" placeholder="e.g. {filename}_{date}" style="flex: 1; border-radius: 4px;">
            `;
        } else if (type === 'Delete') {
            valContainer.innerHTML = `
                <input type="number" class="act-delay" placeholder="Delay in days (0 for instant)" style="flex: 1; border-radius: 4px;" min="0">
            `;
        } else {
            valContainer.innerHTML = `
                <input type="text" class="act-val" disabled placeholder="N/A" style="flex: 1; border-radius: 4px; background: rgba(0,0,0,0.05);">
            `;
        }
    }

    typeSel.addEventListener('change', updateRow);
    updateRow();

    if (actData) {
        typeSel.value = actData.type;
        updateRow();
        if(actData.type === 'Move') row.querySelector('.act-val').value = actData.value.target_folder;
        if(actData.type === 'Rename') row.querySelector('.act-val').value = actData.value.pattern;
        if(actData.type === 'Delete') row.querySelector('.act-delay').value = actData.value.delay_days;
    }
    container.appendChild(row);
}

async function loadRules() {
    try {
        currentRules = await invoke('get_rules');
        renderRules();
    } catch(e) {
        console.error("Failed to fetch rules:", e);
    }
}

function formatBytes(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function renderRules() {
    const container = document.getElementById('rules-container');
    container.innerHTML = ''; 

    currentRules.forEach(rule => {
        const card = document.createElement('div');
        card.className = 'rule-card';
        
        let condDesc = rule.conditions.map(c => {
            const opText = c.value.operator.replace(/([A-Z])/g, ' $1').trim();
            if(c.type === 'Size') return `Size ${opText} ${formatBytes(c.value.bytes)}`;
            return `${c.type} ${opText} '${c.value.text}'`;
        }).join(` <span class="ext-badge">${rule.match_type.toUpperCase()}</span> `);
        
        if (condDesc.length === 0) condDesc = "No conditions";

        let actDesc = rule.actions.map(a => {
            if(a.type === 'Move') return `Move to ${a.value.target_folder}`;
            if(a.type === 'Rename') return `Rename to ${a.value.pattern}`;
            if(a.type === 'Delete') return a.value.delay_days > 0 ? `Delete after ${a.value.delay_days} days` : `Delete instantly`;
            return `Delete`;
        }).join(', ');

        card.innerHTML = `
            <div class="rule-header">
                <div class="rule-title">${rule.name}</div>
                <div class="rule-actions">
                    <svg onclick="window.editRule('${rule.id}')" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="cursor: pointer;"><path d="M12 20h9"></path><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path></svg>
                    <svg onclick="window.deleteRule('${rule.id}')" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="cursor: pointer; margin-left: 8px;"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>
                </div>
            </div>
            <div class="rule-folder" style="margin-bottom: 8px;">
                <strong>IF:</strong> ${condDesc}
            </div>
            <div class="rule-folder">
                <strong>DO:</strong> ${actDesc}
            </div>
        `;
        container.appendChild(card);
    });
}

window.editRule = function(id) {
    const rule = currentRules.find(r => r.id === id);
    if(!rule) return;
    
    editingRuleId = id;
    document.getElementById('rule-name').value = rule.name;
    document.getElementById('rule-match-logic').value = rule.match_type;
    
    document.getElementById('conditions-container').innerHTML = '';
    rule.conditions.forEach(c => addConditionRow(c));
    if(rule.conditions.length === 0) addConditionRow();

    document.getElementById('actions-container').innerHTML = '';
    rule.actions.forEach(a => addActionRow(a));
    if(rule.actions.length === 0) addActionRow();

    document.querySelector('#modal-title').innerText = 'Edit Rule';
    document.getElementById('rule-modal').classList.add('active');
};

window.deleteRule = function(id) {
    window.ruleToDelete = id;
    document.getElementById('delete-modal').classList.add('active');
};

async function loadHistory() {
    try {
        const history = await invoke('get_history');
        const container = document.getElementById('activity-container');
        container.innerHTML = '';
        if (history.length === 0) {
            container.innerHTML = '<p style="color: var(--text-muted); text-align: center; padding: 20px;">No recent activity.</p>';
            return;
        }
        history.forEach(entry => {
            const item = document.createElement('div');
            item.className = 'activity-item';
            const timeStr = new Date(entry.timestamp * 1000).toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'});
            const jsSafeTarget = entry.target_path.replace(/\\/g, '\\\\');
            item.innerHTML = `
                <div class="activity-info">
                    <div class="activity-icon"><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path><polyline points="13 2 13 9 20 9"></polyline></svg></div>
                    <div class="activity-details">
                        <h3>${entry.file_name}</h3>
                        <p>Moved to ${entry.target_path.split('\\\\').pop()} • ${timeStr}</p>
                    </div>
                </div>
                <div class="activity-actions">
                    <button class="btn-icon" title="Undo Move" onclick="window.undoMove('${entry.id}')"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 7v6h6"></path><path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13"></path></svg></button>
                    <button class="btn-icon" title="Open Folder" onclick="window.openFolder('${jsSafeTarget}')"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg></button>
                </div>
            `;
            container.appendChild(item);
        });
    } catch(e) { console.error(e); }
}

window.undoMove = async function(id) {
    try { await invoke('undo_move', { id }); loadHistory(); } catch(e) { alert("Failed to undo: " + e); }
}
window.openFolder = async function(path) {
    try { await invoke('open_folder', { path }); } catch(e) { console.error(e); }
}
