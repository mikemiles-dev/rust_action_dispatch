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

            pagination = "<div id=\"pagination-controls\" style=\"margin-top: 20px;\"></div>";

            container.innerHTML = table + pagination;

            renderPaginationControls(data.page || 1);

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

function renderPaginationControls(currentPage = 1) {
    const container = document.getElementById("pagination-controls");
    if (!container) {
        // Create the container if it doesn't exist
        const newContainer = document.createElement("div");
        newContainer.id = "pagination-controls";
        newContainer.style.marginTop = "20px";
        document.body.appendChild(newContainer);
    }

    const controls = document.getElementById("pagination-controls");
    controls.innerHTML = `
        <button id="prev-page" ${currentPage <= 1 ? "disabled" : ""}>Previous</button>
        <span>Page ${currentPage}</span>
        <button id="next-page">Next</button>
    `;

    document.getElementById("prev-page").onclick = function() {
        if (currentPage > 1) {
            renderRunsTable({ page: currentPage - 1 });
            renderPaginationControls(currentPage - 1);
        }
    };
    document.getElementById("next-page").onclick = function() {
        renderRunsTable({ page: currentPage + 1 });
        renderPaginationControls(currentPage + 1);
    };
}
