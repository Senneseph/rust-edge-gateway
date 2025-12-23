// Rust Edge Gateway Admin UI

const API_BASE = '/api';

const DEFAULT_HANDLER = `//! Handler for this endpoint
use rust_edge_gateway_sdk::prelude::*;

/// Handle incoming requests
fn handle(req: Request) -> Response {
    Response::ok(json!({
        "message": "Hello, World!",
        "path": req.path,
        "method": req.method
    }))
}

handler_loop!(handle);
`;

let editor = null;

function app() {
    return {
        view: 'endpoints',
        endpoints: [],
        services: [],
        apiKeys: [],
        currentEndpoint: {
            id: null,
            name: '',
            domain: '',
            path: '',
            method: 'GET',
            code: DEFAULT_HANDLER,
            compiled: false,
            enabled: false
        },
        currentService: {
            id: null,
            name: '',
            service_type: 'postgres',
            config: {},
            configJson: '{}',
            enabled: true
        },
        currentApiKey: {
            id: null,
            label: '',
            key: '',
            enabled: true,
            permissions: [],
            expires_days: 0,
            created_at: '',
            expires_at: ''
        },
        availablePermissions: [
            'admin:read',
            'admin:write',
            'admin:delete',
            'endpoints:read',
            'endpoints:write',
            'endpoints:delete',
            'services:read',
            'services:write',
            'services:delete',
            'api-keys:read',
            'api-keys:write',
            'api-keys:delete'
        ],
        
        // Import state
        importFile: null,
        importOptions: {
            domain: '',
            domain_id: '',
            create_collection: false,
            compile: true,
            start: false
        },
        importResult: null,
        importing: false,
        dragover: false,

        loading: false,
        message: '',
        messageType: 'success',

        async init() {
            await this.loadEndpoints();
            await this.loadServices();
            await this.loadApiKeys();
            this.$watch('view', (val) => {
                if (val === 'endpoint-editor') {
                    this.$nextTick(() => this.initEditor());
                }
            });
        },

        async login() {
            this.loginError = '';
            try {
                const res = await fetch('/auth/login', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(this.loginData)
                });
                 
                const data = await res.json();
                if (data.success) {
                    if (data.requires_password_change) {
                        // Redirect to password change page
                        window.location.href = '/auth/change-password';
                    } else {
                        // Redirect to admin dashboard
                        window.location.href = '/admin';
                    }
                } else {
                    this.loginError = data.error || 'Login failed';
                }
            } catch (e) {
                this.loginError = 'Network error: ' + e.message;
            }
        },
        
        async changePassword() {
            this.passwordChangeError = '';
            this.passwordChangeSuccess = '';
            
            if (this.passwordChangeData.newPassword !== this.passwordChangeData.confirmPassword) {
                this.passwordChangeError = 'New passwords do not match';
                return;
            }
            
            try {
                const res = await fetch('/auth/change-password', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        username: 'admin',
                        current_password: this.passwordChangeData.currentPassword,
                        new_password: this.passwordChangeData.newPassword
                    })
                });
                
                const data = await res.json();
                if (data.success) {
                    this.passwordChangeSuccess = 'Password changed successfully! Redirecting to admin panel...';
                    setTimeout(() => {
                        window.location.href = '/admin';
                    }, 2000);
                } else {
                    this.passwordChangeError = data.message || 'Password change failed';
                }
            } catch (e) {
                this.passwordChangeError = 'Network error: ' + e.message;
            }
        },

        switchView(newView) {
            this.message = '';
            this.view = newView;
        },

        async loadEndpoints() {
            this.loading = true;
            try {
                const res = await fetch(`${API_BASE}/endpoints`);
                const data = await res.json();
                if (data.ok || data.success) {
                    this.endpoints = data.data || [];
                }
            } catch (e) {
                console.error('Failed to load endpoints:', e);
            }
            this.loading = false;
        },

        async loadServices() {
            try {
                const res = await fetch(`${API_BASE}/services`);
                const data = await res.json();
                if (data.ok || data.success) {
                    this.services = data.data || [];
                }
            } catch (e) {
                console.error('Failed to load services:', e);
            }
        },

        async loadApiKeys() {
            this.loading = true;
            try {
                const res = await fetch('/admin/api-keys');
                const data = await res.json();
                if (data.ok || data.success) {
                    this.apiKeys = data.data || [];
                }
            } catch (e) {
                console.error('Failed to load API keys:', e);
            }
            this.loading = false;
        },

        initEditor() {
            if (editor) {
                editor.setValue(this.currentEndpoint.code || DEFAULT_HANDLER);
                return;
            }

            require.config({ paths: { vs: 'https://cdn.jsdelivr.net/npm/monaco-editor@0.45.0/min/vs' } });
            require(['vs/editor/editor.main'], () => {
                editor = monaco.editor.create(document.getElementById('editor'), {
                    value: this.currentEndpoint.code || DEFAULT_HANDLER,
                    language: 'rust',
                    theme: 'vs-dark',
                    minimap: { enabled: false },
                    automaticLayout: true,
                    fontSize: 14
                });
            });
        },

        // ===== Endpoint Methods =====
        newEndpoint() {
            this.currentEndpoint = {
                id: null,
                name: '',
                domain: '',
                path: '',
                method: 'GET',
                code: DEFAULT_HANDLER,
                compiled: false,
                enabled: false
            };
            this.message = '';
            this.view = 'endpoint-editor';
        },

        async editEndpoint(ep) {
            this.currentEndpoint = { ...ep };
            if (ep.id) {
                try {
                    const res = await fetch(`${API_BASE}/endpoints/${ep.id}/code`);
                    const data = await res.json();
                    if (data.ok || data.success) {
                        this.currentEndpoint.code = data.data || DEFAULT_HANDLER;
                    }
                } catch (e) {
                    console.error('Failed to load code:', e);
                }
            }
            this.message = '';
            this.view = 'endpoint-editor';
        },

        async saveEndpoint() {
            const code = editor ? editor.getValue() : this.currentEndpoint.code;
            this.currentEndpoint.code = code;

            try {
                let res;
                if (this.currentEndpoint.id) {
                    res = await fetch(`${API_BASE}/endpoints/${this.currentEndpoint.id}`, {
                        method: 'PUT',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify(this.currentEndpoint)
                    });
                    await fetch(`${API_BASE}/endpoints/${this.currentEndpoint.id}/code`, {
                        method: 'PUT',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ code })
                    });
                } else {
                    res = await fetch(`${API_BASE}/endpoints`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ ...this.currentEndpoint, code })
                    });
                }

                const data = await res.json();
                if (data.ok || data.success) {
                    this.currentEndpoint = data.data;
                    this.showMessage('Saved successfully!', 'success');
                    await this.loadEndpoints();
                } else {
                    this.showMessage(data.error || 'Save failed', 'error');
                }
            } catch (e) {
                this.showMessage('Failed to save: ' + e.message, 'error');
            }
        },

        async compileEndpoint() {
            if (!this.currentEndpoint.id) return;

            this.showMessage('Compiling...', 'success');
            try {
                const res = await fetch(`${API_BASE}/endpoints/${this.currentEndpoint.id}/compile`, { method: 'POST' });
                const data = await res.json();
                if (data.ok || data.success) {
                    this.currentEndpoint.compiled = true;
                    this.showMessage('Compiled successfully!', 'success');
                    await this.loadEndpoints();
                } else {
                    this.showMessage(data.error || 'Compilation failed', 'error');
                }
            } catch (e) {
                this.showMessage('Compilation failed: ' + e.message, 'error');
            }
        },

        async toggleEndpoint() {
            if (!this.currentEndpoint.id) return;
            const action = this.currentEndpoint.enabled ? 'stop' : 'start';

            try {
                const res = await fetch(`${API_BASE}/endpoints/${this.currentEndpoint.id}/${action}`, { method: 'POST' });
                const data = await res.json();
                if (data.ok || data.success) {
                    this.currentEndpoint.enabled = !this.currentEndpoint.enabled;
                    this.showMessage(`Endpoint ${action}ed!`, 'success');
                    await this.loadEndpoints();
                } else {
                    this.showMessage(data.error || `Failed to ${action}`, 'error');
                }
            } catch (e) {
                this.showMessage(`Failed to ${action}: ` + e.message, 'error');
            }
        },

        async deleteEndpoint(id) {
            if (!confirm('Delete this endpoint?')) return;

            try {
                const res = await fetch(`${API_BASE}/endpoints/${id}`, { method: 'DELETE' });
                const data = await res.json();
                if (data.ok || data.success) {
                    await this.loadEndpoints();
                }
            } catch (e) {
                console.error('Delete failed:', e);
            }
        },

        // ===== Service Methods =====
        newService() {
            this.currentService = {
                id: null,
                name: '',
                service_type: 'postgres',
                config: {},
                configJson: '{\n  "host": "localhost",\n  "port": 5432,\n  "database": "mydb"\n}',
                enabled: true
            };
            this.message = '';
            this.view = 'service-editor';
        },

        async editService(svc) {
            this.currentService = {
                ...svc,
                configJson: JSON.stringify(svc.config || {}, null, 2)
            };
            this.message = '';
            this.view = 'service-editor';
        },

        async saveService() {
            try {
                this.currentService.config = JSON.parse(this.currentService.configJson);
            } catch (e) {
                this.showMessage('Invalid JSON in configuration', 'error');
                return;
            }

            try {
                let res;
                const payload = {
                    name: this.currentService.name,
                    service_type: this.currentService.service_type,
                    config: this.currentService.config,
                    enabled: this.currentService.enabled
                };

                if (this.currentService.id) {
                    res = await fetch(`${API_BASE}/services/${this.currentService.id}`, {
                        method: 'PUT',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify(payload)
                    });
                } else {
                    res = await fetch(`${API_BASE}/services`, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify(payload)
                    });
                }

                const data = await res.json();
                if (data.ok || data.success) {
                    this.currentService = { ...data.data, configJson: JSON.stringify(data.data.config || {}, null, 2) };
                    this.showMessage('Service saved!', 'success');
                    await this.loadServices();
                } else {
                    this.showMessage(data.error || 'Save failed', 'error');
                }
            } catch (e) {
                this.showMessage('Failed to save: ' + e.message, 'error');
            }
        },

        async testService(id) {
            if (!id) return;

            this.showMessage('Testing connection...', 'success');
            try {
                const res = await fetch(`${API_BASE}/services/${id}/test`, { method: 'POST' });
                const data = await res.json();
                if (data.ok || data.success) {
                    this.showMessage('Connection successful!', 'success');
                } else {
                    this.showMessage(data.error || 'Connection failed', 'error');
                }
            } catch (e) {
                this.showMessage('Test failed: ' + e.message, 'error');
            }
        },

        async deleteService(id) {
            if (!confirm('Delete this service?')) return;

            try {
                const res = await fetch(`${API_BASE}/services/${id}`, { method: 'DELETE' });
                const data = await res.json();
                if (data.ok || data.success) {
                    await this.loadServices();
                    this.showMessage('Service deleted', 'success');
                }
            } catch (e) {
                console.error('Delete failed:', e);
            }
        },

        // ===== Import Methods =====
        handleDrop(e) {
            this.dragover = false;
            const files = e.dataTransfer?.files;
            if (files && files.length > 0) {
                this.importFile = files[0];
            }
        },

        handleFileSelect(e) {
            const files = e.target?.files;
            if (files && files.length > 0) {
                this.importFile = files[0];
            }
        },

        async importBundle() {
            if (!this.importFile) {
                this.showMessage('Please select a file', 'error');
                return;
            }
            if (!this.importOptions.domain) {
                this.showMessage('Please enter a domain', 'error');
                return;
            }

            this.importing = true;
            this.importResult = null;
            this.message = '';

            try {
                const formData = new FormData();
                formData.append('bundle', this.importFile);

                const params = new URLSearchParams();
                params.set('domain', this.importOptions.domain);
                if (this.importOptions.domain_id) params.set('domain_id', this.importOptions.domain_id);
                if (this.importOptions.create_collection) params.set('create_collection', 'true');
                if (this.importOptions.compile) params.set('compile', 'true');
                if (this.importOptions.start) params.set('start', 'true');

                const res = await fetch(`${API_BASE}/import/bundle?${params.toString()}`, {
                    method: 'POST',
                    body: formData
                });

                const data = await res.json();
                if (data.ok || data.success) {
                    this.importResult = data.data;
                    this.showMessage('Import successful!', 'success');
                    await this.loadEndpoints();
                } else {
                    this.showMessage(data.error || 'Import failed', 'error');
                }
            } catch (e) {
                this.showMessage('Import failed: ' + e.message, 'error');
            }

            this.importing = false;
        },

        // ===== API Key Methods =====
        newApiKey() {
            this.currentApiKey = {
                id: null,
                label: '',
                key: '',
                enabled: true,
                permissions: [],
                expires_days: 0,
                created_at: '',
                expires_at: ''
            };
            this.message = '';
            this.view = 'api-key-editor';
        },

        async saveApiKey() {
            try {
                const res = await fetch('/admin/api-keys', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(this.currentApiKey)
                });

                const data = await res.json();
                if (data.ok || data.success) {
                    this.currentApiKey = data.data;
                    this.showMessage('API key generated successfully!', 'success');
                    await this.loadApiKeys();
                    this.view = 'api-keys';
                } else {
                    this.showMessage(data.error || 'Failed to generate API key', 'error');
                }
            } catch (e) {
                this.showMessage('Failed to generate API key: ' + e.message, 'error');
            }
        },

        async copyApiKey(key) {
            try {
                await navigator.clipboard.writeText(key);
                this.showMessage('API key copied to clipboard!', 'success');
            } catch (e) {
                this.showMessage('Failed to copy to clipboard', 'error');
            }
        },

        // Login state
        loginData: {
            username: 'admin',
            password: ''
        },
        loginError: '',
        
        // Password change state
        passwordChangeData: {
            currentPassword: '',
            newPassword: '',
            confirmPassword: ''
        },
        passwordChangeError: '',
        passwordChangeSuccess: '',

        async enableApiKey(key) {
            try {
                const res = await fetch(`/admin/api-keys/${key}/enable`, {
                    method: 'POST'
                });
                const data = await res.json();
                if (data.ok || data.success) {
                    this.showMessage('API key enabled!', 'success');
                    await this.loadApiKeys();
                } else {
                    this.showMessage(data.error || 'Failed to enable API key', 'error');
                }
            } catch (e) {
                this.showMessage('Failed to enable API key: ' + e.message, 'error');
            }
        },

        async disableApiKey(key) {
            try {
                const res = await fetch(`/admin/api-keys/${key}/disable`, {
                    method: 'POST'
                });
                const data = await res.json();
                if (data.ok || data.success) {
                    this.showMessage('API key disabled!', 'success');
                    await this.loadApiKeys();
                } else {
                    this.showMessage(data.error || 'Failed to disable API key', 'error');
                }
            } catch (e) {
                this.showMessage('Failed to disable API key: ' + e.message, 'error');
            }
        },

        async deleteApiKey(id) {
            if (!confirm('Delete this API key? This cannot be undone.')) return;

            try {
                const res = await fetch(`/admin/api-keys/${id}`, { method: 'DELETE' });
                const data = await res.json();
                if (data.ok || data.success) {
                    await this.loadApiKeys();
                }
            } catch (e) {
                console.error('Delete failed:', e);
            }
        },

        showMessage(msg, type) {
            this.message = msg;
            this.messageType = type;
        }
    };
}
