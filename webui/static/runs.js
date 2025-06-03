function renderRunsTable(params = {}) {
    // Append filter string to the URL if provided
    const url = "/runs_data";
    AjaxUtils.getJsonData(url, params)
        .then(data => {
            const container = document.getElementById("items");
            if (!container) return;

            let current_page = data.current_page;
            let total_pages = data.total_pages;

            data = data.items;

            // Assume data is an array of objects
            if (!Array.isArray(data) || data.length === 0) {
                container.innerHTML = '<p>No data available.</p>';
                return;
            }

            // Get table headers from object keys
            let table = '<table><thead><tr>';
            table += `<th><a href=\"#\" class=\"sort_column\" onclick=\"FilterUtils.applyFilterAndReload('sort', 'job_name', true); return false;\">Job Name</a></th>`;
            table += `<th><a href=\"#\" class=\"sort_column\" onclick=\"FilterUtils.applyFilterAndReload('sort', 'agent_name', true); return false;\">Agent Name</a></th>`;
            table += `<th><a href=\"#\" class=\"sort_column\" onclick=\"FilterUtils.applyFilterAndReload('sort', 'started_at', true); return false;\">Started At</a></th>`;
            table += `<th><a href=\"#\" class=\"sort_column\" onclick=\"FilterUtils.applyFilterAndReload('sort', 'completed_at', true); return false;\">Completed At</a></th>`;
            table += `<th><a href=\"#\" class=\"sort_column\" onclick=\"FilterUtils.applyFilterAndReload('sort', 'completed_at', true); return false;\">Return Code</a></th>`;
            table += '</tr></thead><tbody>';

            // Add table rows
            data.forEach(item => {
                let start_at_value = item["started_at"].$date.$numberLong;
                let completed_at_value = item["completed_at"].$date.$numberLong;
                table += '<tr>';
                table += `<td>${item["job_name"]}</td>`;
                table += `<td>${item["agent_name"]}</td>`;
                table += `<td class="utc-date" data-timestamp="${start_at_value}">${start_at_value}</td>`;
                table += `<td class="utc-date" data-timestamp="${completed_at_value}">${completed_at_value}</td>`;
                table += `<td>${item["return_code"]}</td>`;
                table += '</tr>';
            });

            table += '</tbody></table>';

            pagination = "<div class=\"pagination_controls\" id=\"pagination-controls\" style=\"margin-top: 20px;\"></div>";

            container.innerHTML = table + pagination;

            renderPaginationControls(current_page, total_pages);

            DateTimeUtils.convertUtcDateElements();

            // Auto-refresh the table every 10 seconds
            setTimeout(() => renderRunsTable(params), 10000);
        })
        .catch(error => {
            const container = document.getElementById("items");
            if (container) {
                container.innerHTML = `<p>Error loading data: ${error.message}</p>`;
            }
        });
}
