// Off-context Admin Interface JavaScript

class OffContextAdmin {
    constructor() {
        this.apiBase = '';
        this.init();
    }

    async init() {
        await this.loadStatus();
        this.setupEventListeners();
        
        // Auto-refresh status every 30 seconds
        setInterval(() => this.loadStatus(), 30000);
    }

    setupEventListeners() {
        // Search functionality
        const searchBtn = document.getElementById('search-btn');
        const searchInput = document.getElementById('search-input');
        
        searchBtn.addEventListener('click', () => this.performSearch());
        searchInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                this.performSearch();
            }
        });

        // Export functionality
        const exportBtn = document.getElementById('export-btn');
        exportBtn.addEventListener('click', () => this.performExport());

        // Management actions
        const initBtn = document.getElementById('init-btn');
        const clearBtn = document.getElementById('clear-btn');
        const resetBtn = document.getElementById('reset-btn');
        
        initBtn.addEventListener('click', () => this.performInit());
        clearBtn.addEventListener('click', () => this.performClear());
        resetBtn.addEventListener('click', () => this.performReset());
    }

    async loadStatus() {
        try {
            const response = await fetch('/api/status');
            const data = await response.json();
            this.updateStatusUI(data);
        } catch (error) {
            console.error('Failed to load status:', error);
            this.showError('Failed to load system status');
        }
    }

    updateStatusUI(data) {
        // Update project info in header
        document.getElementById('project-name').textContent = data.project_name;
        document.getElementById('project-path').textContent = data.project_root;

        // Update hooks status
        const hooksStatus = document.getElementById('hooks-status');
        hooksStatus.innerHTML = this.createStatusContent(
            data.hooks_active ? 'Active and configured' : 'Not configured',
            data.hooks_active ? 'success' : 'warning'
        );

        // Update database status
        const dbStatus = document.getElementById('database-status');
        dbStatus.innerHTML = this.createStatusContent(
            data.database_exists 
                ? `${data.conversation_count} conversations, ${data.database_size}`
                : 'Not initialized',
            data.database_exists ? 'success' : 'warning'
        );

        // Update embeddings status
        const embedStatus = document.getElementById('embeddings-status');
        embedStatus.innerHTML = this.createStatusContent(
            data.embeddings_provider,
            data.embeddings_available ? 'success' : 'warning'
        );

        // Update project details section
        this.updateProjectDetails(data);
        
        // Re-initialize Lucide icons after DOM update
        lucide.createIcons();
    }

    updateProjectDetails(data) {
        const detailsContainer = document.getElementById('project-details');
        detailsContainer.innerHTML = `
            <div class="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
                <h3 class="text-lg font-medium text-gray-900 mb-4 flex items-center">
                    <div class="w-6 h-6 bg-blue-500 rounded-lg flex items-center justify-center mr-3">
                        <i data-lucide="folder" class="w-4 h-4 text-white"></i>
                    </div>
                    File Locations
                </h3>
                <div class="space-y-3 text-sm">
                    <div class="flex justify-between">
                        <span class="text-gray-600">Config Directory:</span>
                        <span class="text-gray-900 font-mono text-xs break-all">${data.config_dir}</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-600">Database Path:</span>
                        <span class="text-gray-900 font-mono text-xs break-all">${data.database_path}</span>
                    </div>
                    ${data.hooks_path ? `
                        <div class="flex justify-between">
                            <span class="text-gray-600">Global Hooks:</span>
                            <span class="text-gray-900 font-mono text-xs break-all">${data.hooks_path}</span>
                        </div>
                    ` : ''}
                </div>
            </div>

            <div class="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
                <h3 class="text-lg font-medium text-gray-900 mb-4 flex items-center">
                    <div class="w-6 h-6 bg-green-500 rounded-lg flex items-center justify-center mr-3">
                        <i data-lucide="bar-chart-3" class="w-4 h-4 text-white"></i>
                    </div>
                    Statistics
                </h3>
                <div class="space-y-3 text-sm">
                    <div class="flex justify-between">
                        <span class="text-gray-600">Conversations:</span>
                        <span class="text-green-600 font-semibold">${data.conversation_count}</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-600">Database Size:</span>
                        <span class="text-blue-600 font-semibold">${data.database_size}</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-600">Last Activity:</span>
                        <span class="text-purple-600 font-semibold">${data.last_activity || 'Never'}</span>
                    </div>
                </div>
            </div>
        `;
    }

    createStatusContent(text, status) {
        const statusColors = {
            success: 'text-green-600',
            warning: 'text-yellow-600',
            error: 'text-red-600'
        };
        
        const iconColors = {
            success: 'text-green-600',
            warning: 'text-yellow-600', 
            error: 'text-red-600'
        };

        const icons = {
            success: 'check-circle',
            warning: 'alert-triangle',
            error: 'x-circle'
        };

        return `
            <span class="inline-flex items-center ${statusColors[status]}">
                <i data-lucide="${icons[status]}" class="w-4 h-4 mr-2"></i>
                ${text}
            </span>
        `;
    }

    async performSearch() {
        const query = document.getElementById('search-input').value.trim();
        const limit = document.getElementById('search-limit').value;
        
        if (!query) {
            this.showError('Please enter a search query');
            return;
        }

        const searchBtn = document.getElementById('search-btn');
        const resultsDiv = document.getElementById('search-results');
        
        // Show loading state
        searchBtn.disabled = true;
        searchBtn.innerHTML = `
            <svg class="animate-spin w-4 h-4 mr-2 inline" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="m4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            Searching...
        `;
        resultsDiv.innerHTML = `
            <div class="flex items-center justify-center py-12">
                <svg class="animate-spin w-8 h-8 text-purple-400" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="m4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                <span class="ml-3 text-gray-300">Searching conversations...</span>
            </div>
        `;

        try {
            const response = await fetch(`/api/search?q=${encodeURIComponent(query)}&limit=${limit}`);
            const data = await response.json();
            this.displaySearchResults(data);
        } catch (error) {
            console.error('Search failed:', error);
            this.showError('Search failed. Please try again.');
        } finally {
            searchBtn.disabled = false;
            searchBtn.textContent = 'Search';
        }
    }

    displaySearchResults(data) {
        const resultsDiv = document.getElementById('search-results');
        
        if (data.results.length === 0) {
            resultsDiv.innerHTML = `
                <div class="text-center py-12">
                    <div class="w-16 h-16 bg-gray-200 rounded-full flex items-center justify-center mx-auto mb-4">
                        <i data-lucide="search-x" class="w-8 h-8 text-gray-500"></i>
                    </div>
                    <h3 class="text-xl font-semibold mb-2 text-gray-700">No results found</h3>
                    <p class="text-gray-500">Try different keywords or check if conversations have been imported.</p>
                </div>
            `;
            return;
        }

        const resultsHTML = data.results.map((result, index) => `
            <div class="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow duration-200 mb-4">
                <div class="flex justify-between items-start mb-4">
                    <div class="flex-1">
                        <h3 class="text-lg font-medium text-gray-900 mb-1">
                            <span class="text-blue-600">#${index + 1}</span>
                            Conversation Result
                        </h3>
                        <p class="text-gray-500 text-sm">${result.timestamp}</p>
                    </div>
                    <div class="bg-blue-500 text-white px-3 py-1 rounded-full text-sm font-medium">
                        Score: ${result.score.toFixed(2)}
                    </div>
                </div>
                
                <div class="bg-gray-50 rounded-lg p-4 mb-4 border-l-4 border-blue-500">
                    <div class="text-gray-800 text-sm whitespace-pre-wrap font-mono overflow-x-auto">${result.highlighted_snippet || this.escapeHtml(result.snippet)}</div>
                </div>
                
                <div class="flex flex-wrap gap-3 text-sm">
                    ${result.project_path ? `
                        <span class="inline-flex items-center bg-blue-50 text-blue-700 px-2 py-1 rounded-lg border border-blue-200">
                            <i data-lucide="folder" class="w-3 h-3 mr-1"></i> ${result.project_path}
                        </span>
                    ` : ''}
                    ${result.tags.length > 0 ? `
                        <span class="inline-flex items-center bg-green-50 text-green-700 px-2 py-1 rounded-lg border border-green-200">
                            <i data-lucide="tag" class="w-3 h-3 mr-1"></i> ${result.tags.join(', ')}
                        </span>
                    ` : ''}
                    <span class="inline-flex items-center bg-gray-50 text-gray-700 px-2 py-1 rounded-lg border border-gray-200">
                        <i data-lucide="message-circle" class="w-3 h-3 mr-1"></i> ${result.token_count} tokens
                    </span>
                </div>
            </div>
        `).join('');

        resultsDiv.innerHTML = `
            <div class="mb-6 p-4 bg-blue-50 rounded-lg border border-blue-200">
                <p class="text-gray-900 font-medium">
                    Found <span class="text-blue-600">${data.results.length}</span> results
                    <span class="text-gray-500 text-sm ml-2">(of ${data.total_conversations} total conversations)</span>
                </p>
            </div>
            ${resultsHTML}
        `;
        
        // Re-initialize Lucide icons for search results
        lucide.createIcons();
    }

    async performExport() {
        const format = document.getElementById('export-format').value;
        const exportBtn = document.getElementById('export-btn');
        const resultDiv = document.getElementById('export-result');

        // Show loading state
        exportBtn.disabled = true;
        exportBtn.innerHTML = `
            <svg class="animate-spin w-4 h-4 mr-2 inline" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="m4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            Generating...
        `;
        resultDiv.innerHTML = `
            <div class="flex items-center justify-center py-12">
                <svg class="animate-spin w-8 h-8 text-emerald-400" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="m4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                <span class="ml-3 text-gray-300">Generating export...</span>
            </div>
        `;

        try {
            const response = await fetch('/api/export', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ format }),
            });

            const data = await response.json();
            this.displayExportResult(data);
        } catch (error) {
            console.error('Export failed:', error);
            this.showError('Export failed. Please try again.');
        } finally {
            exportBtn.disabled = false;
            exportBtn.textContent = 'Generate Export';
        }
    }

    displayExportResult(data) {
        const resultDiv = document.getElementById('export-result');
        const filename = `conversations.${data.format}`;
        
        if (data.conversation_count === 0) {
            resultDiv.innerHTML = `
                <div class="bg-yellow-50 border border-yellow-200 rounded-lg p-6">
                    <div class="flex items-center mb-4">
                        <i data-lucide="alert-triangle" class="w-6 h-6 text-yellow-600 mr-3"></i>
                        <div>
                            <h3 class="text-lg font-medium text-gray-900">No Conversations Found</h3>
                            <p class="text-gray-600 text-sm">There are no conversations to export. Try importing conversations first with 'off-context import'.</p>
                        </div>
                    </div>
                </div>
            `;
            return;
        }
        
        resultDiv.innerHTML = `
            <div class="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
                <div class="flex justify-between items-center mb-6">
                    <div>
                        <h3 class="text-xl font-medium text-gray-900 mb-1">${data.format.toUpperCase()} Export</h3>
                        <p class="text-gray-600 text-sm">${data.conversation_count} conversations exported successfully</p>
                    </div>
                    <button 
                        onclick="app.downloadExport('${filename}', ${JSON.stringify(data.content).replace(/"/g, '&quot;')})"
                        class="px-6 py-2 bg-green-600 text-white font-medium rounded-lg hover:bg-green-700 transition-colors duration-200 shadow-sm flex items-center"
                    >
                        <i data-lucide="download" class="w-4 h-4 mr-2"></i>
                        Download File
                    </button>
                </div>
                <div class="bg-gray-50 rounded-lg border border-gray-200 overflow-hidden">
                    <div class="bg-gray-100 px-4 py-2 border-b border-gray-200">
                        <span class="text-gray-700 text-sm font-mono">${filename}</span>
                    </div>
                    <textarea 
                        readonly 
                        class="w-full h-80 p-4 bg-gray-50 text-gray-800 font-mono text-sm resize-none border-none outline-none"
                    >${data.content}</textarea>
                </div>
            </div>
        `;
    }

    downloadExport(filename, content) {
        const blob = new Blob([content], { type: 'text/plain' });
        const url = window.URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = filename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        window.URL.revokeObjectURL(url);
    }

    async performInit() {
        const confirmed = await this.showConfirmModal(
            'Initialize Project',
            'Set up off-context integration for this project?',
            'This will configure Claude Code hooks and create the project configuration directory.',
            'Initialize',
            'default'
        );
        
        if (!confirmed) {
            return;
        }

        const initBtn = document.getElementById('init-btn');
        const originalText = initBtn.textContent;
        
        initBtn.disabled = true;
        initBtn.innerHTML = `
            <svg class="animate-spin w-4 h-4 mr-2 inline" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="m4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            Initializing...
        `;

        try {
            const response = await fetch('/api/init', { method: 'POST' });
            const data = await response.json();
            
            if (data.success) {
                this.showSuccessModal(
                    'Project Initialized Successfully',
                    'Off-context integration has been configured for this project.',
                    [
                        'Claude Code hooks configured in .claude/settings.local.json',
                        'Project configuration directory created at .off-context/',
                        'Project configuration initialized',
                        'Start using Claude Code normally to build conversation history'
                    ]
                );
                await this.loadStatus(); // Refresh status
            } else {
                this.showError(data.message || 'Failed to initialize project');
            }
        } catch (error) {
            console.error('Init failed:', error);
            this.showError('Initialization failed. Please try again.');
        } finally {
            initBtn.disabled = false;
            initBtn.textContent = originalText;
        }
    }

    async performClear() {
        const confirmed = await this.showConfirmModal(
            'Clear Project Hooks',
            'Are you sure you want to clear hooks from this project?',
            'This will remove off-context integration from the current project only.',
            'Clear Hooks',
            'cancel'
        );
        
        if (!confirmed) {
            return;
        }

        const clearBtn = document.getElementById('clear-btn');
        const originalText = clearBtn.textContent;
        
        clearBtn.disabled = true;
        clearBtn.innerHTML = `
            <svg class="animate-spin w-4 h-4 mr-2 inline" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="m4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            Clearing...
        `;

        try {
            const response = await fetch('/api/clear', { method: 'POST' });
            const data = await response.json();
            
            if (data.success) {
                this.showSuccessModal(
                    'Hooks Cleared Successfully',
                    'Project hooks have been removed from this project.',
                    [
                        'The off-context integration has been disabled for this project',
                        'Your conversation history remains intact',
                        'Re-run "off-context init" to re-enable integration'
                    ]
                );
                await this.loadStatus(); // Refresh status
            } else {
                this.showError(data.message || 'Failed to clear hooks');
            }
        } catch (error) {
            console.error('Clear failed:', error);
            this.showError('Clear operation failed');
        } finally {
            clearBtn.disabled = false;
            clearBtn.textContent = originalText;
        }
    }

    async performReset() {
        const confirmed = await this.showConfirmModal(
            'Reset Memory Database',
            'DANGER: This will permanently delete all conversation history!',
            'This action cannot be undone. All stored conversations and search indices will be lost.',
            'Reset Database',
            'destructive'
        );
        
        if (!confirmed) {
            return;
        }

        const resetBtn = document.getElementById('reset-btn');
        const originalText = resetBtn.textContent;
        
        resetBtn.disabled = true;
        resetBtn.innerHTML = `
            <svg class="animate-spin w-4 h-4 mr-2 inline" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="m4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            Resetting...
        `;

        try {
            const response = await fetch('/api/reset', { method: 'POST' });
            const data = await response.json();
            
            if (data.success) {
                this.showSuccessModal(
                    'Memory Reset Complete',
                    'The memory database has been successfully reset.',
                    [
                        'Project conversation history deleted',
                        'Project embeddings and search indices cleared',
                        'Run "off-context init" to reinitialize the project',
                        'Run "off-context import" to reimport conversations'
                    ]
                );
                await this.loadStatus(); // Refresh status
                
                // Clear search results since database is now empty
                document.getElementById('search-results').innerHTML = '';
            } else {
                this.showError(data.message || 'Failed to reset database');
            }
        } catch (error) {
            console.error('Reset failed:', error);
            this.showError('Reset operation failed');
        } finally {
            resetBtn.disabled = false;
            resetBtn.textContent = originalText;
        }
    }

    showSuccess(message) {
        // Enhanced success notification
        const notification = document.createElement('div');
        notification.className = 'fixed top-4 right-4 bg-green-600 text-white px-6 py-3 rounded-lg shadow-lg z-50';
        notification.innerHTML = `
            <div class="flex items-center">
                <i data-lucide="check-circle" class="w-5 h-5 mr-3"></i>
                <span>${message}</span>
            </div>
        `;
        
        document.body.appendChild(notification);
        setTimeout(() => {
            if (document.body.contains(notification)) {
                document.body.removeChild(notification);
            }
        }, 4000);
    }

    showError(message) {
        // Enhanced error notification  
        const notification = document.createElement('div');
        notification.className = 'fixed top-4 right-4 bg-red-600 text-white px-6 py-3 rounded-lg shadow-lg z-50';
        notification.innerHTML = `
            <div class="flex items-center">
                <i data-lucide="x-circle" class="w-5 h-5 mr-3"></i>
                <span>${message}</span>
            </div>
        `;
        
        document.body.appendChild(notification);
        setTimeout(() => {
            if (document.body.contains(notification)) {
                document.body.removeChild(notification);
            }
        }, 5000);
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    // Modal functions
    showConfirmModal(title, message, details, actionText, type = 'default') {
        return new Promise((resolve) => {
            const overlay = document.getElementById('modal-overlay');
            const content = document.getElementById('modal-content');
            
            const buttonColors = {
                'default': 'bg-gcp-blue hover:bg-blue-600',
                'cancel': 'bg-orange-500 hover:bg-orange-600', 
                'destructive': 'bg-gcp-red hover:bg-red-600'
            };
            
            const iconColors = {
                'default': 'text-gcp-blue',
                'cancel': 'text-orange-500',
                'destructive': 'text-gcp-red'
            };
            
            const icons = {
                'default': 'help-circle',
                'cancel': 'alert-triangle',
                'destructive': 'alert-octagon'
            };
            
            content.innerHTML = `
                <div class="p-6">
                    <div class="flex items-center mb-4">
                        <div class="w-12 h-12 rounded-full bg-gray-100 flex items-center justify-center mr-4">
                            <i data-lucide="${icons[type]}" class="w-6 h-6 ${iconColors[type]}"></i>
                        </div>
                        <div class="flex-1">
                            <h3 class="text-lg font-medium text-gray-900">${title}</h3>
                            <p class="text-gray-600 text-sm mt-1">${message}</p>
                        </div>
                    </div>
                    <div class="bg-gray-50 rounded-lg p-3 mb-6">
                        <p class="text-gray-700 text-sm">${details}</p>
                    </div>
                    <div class="flex justify-end space-x-3">
                        <button 
                            id="modal-cancel" 
                            class="px-4 py-2 text-gray-600 hover:text-gray-800 font-medium transition-colors duration-200"
                        >
                            Cancel
                        </button>
                        <button 
                            id="modal-confirm" 
                            class="px-6 py-2 ${buttonColors[type]} text-white font-medium rounded-lg transition-colors duration-200"
                        >
                            ${actionText}
                        </button>
                    </div>
                </div>
            `;
            
            // Add event listeners
            document.getElementById('modal-cancel').addEventListener('click', () => {
                this.hideModal();
                resolve(false);
            });
            
            document.getElementById('modal-confirm').addEventListener('click', () => {
                this.hideModal();
                resolve(true);
            });
            
            // Show modal
            overlay.classList.remove('hidden');
            lucide.createIcons();
        });
    }

    showSuccessModal(title, message, steps) {
        const overlay = document.getElementById('modal-overlay');
        const content = document.getElementById('modal-content');
        
        const stepsHtml = steps.map(step => `
            <li class="flex items-start">
                <i data-lucide="check" class="w-4 h-4 text-gcp-green mr-3 mt-0.5 flex-shrink-0"></i>
                <span>${step}</span>
            </li>
        `).join('');
        
        content.innerHTML = `
            <div class="p-6">
                <div class="flex items-center mb-4">
                    <div class="w-12 h-12 rounded-full bg-green-100 flex items-center justify-center mr-4">
                        <i data-lucide="check-circle" class="w-6 h-6 text-gcp-green"></i>
                    </div>
                    <div class="flex-1">
                        <h3 class="text-lg font-medium text-gray-900">${title}</h3>
                        <p class="text-gray-600 text-sm mt-1">${message}</p>
                    </div>
                </div>
                <div class="bg-green-50 rounded-lg p-4 mb-6">
                    <h4 class="font-medium text-gray-900 mb-3">Next Steps:</h4>
                    <ul class="space-y-2 text-sm text-gray-700">
                        ${stepsHtml}
                    </ul>
                </div>
                <div class="flex justify-end">
                    <button 
                        id="modal-close" 
                        class="px-6 py-2 bg-gcp-green text-white font-medium rounded-lg hover:bg-green-600 transition-colors duration-200"
                    >
                        Got it
                    </button>
                </div>
            </div>
        `;
        
        document.getElementById('modal-close').addEventListener('click', () => {
            this.hideModal();
        });
        
        overlay.classList.remove('hidden');
        lucide.createIcons();
    }

    hideModal() {
        const overlay = document.getElementById('modal-overlay');
        overlay.classList.add('hidden');
    }
}

// Initialize the application when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.app = new OffContextAdmin();
});