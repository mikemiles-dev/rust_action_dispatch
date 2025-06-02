function renderRunsTable(params = {}) {
    // Append filter string to the URL if provided
    const url = "/runs_data";
    AjaxUtils.getJsonData(url, params)
        .then(data => {
            const container = document.getElementById("items");
            if (!container) return;

            data = data.items;

            // Assume data is an array of objects
            if (!Array.isArray(data) || data.length === 0) {
                container.innerHTML = '<p>No data available.</p>';
                return;
            }

            // Get table headers from object keys
            let table = '<table><thead><tr>';
            table += `<th><a href=\"#\" class=\"sort_column\" onclick=\"FilterUtils.applyFilterAndReload('sort', 'job_name', true); return false;\">Job Name</a></th>`;
            table += `<th>Agent Name</th>`;
            table += `<th>Started At</th>`;
            table += `<th>Completed At</th>`;
            table += `<th>Return Code</th>`;
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
            container.innerHTML = table;

            DateTimeUtils.convertUtcDateElements();

            // Set input time for specific elements if they exist            

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