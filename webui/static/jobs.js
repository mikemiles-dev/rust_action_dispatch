
function renderJobsTable(params = {}) {
    // Append filter string to the URL if provided
    const url = "/jobs_data";
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
            } else {
                // Get table headers from object keys
                let table = '<table><thead><tr>';
                table += '<th><input type="checkbox" class="item-checkbox"></th>'
                table += `<th><a href=\"#\" class=\"sort_column\" onclick=\"FilterUtils.applyFilterAndReload('sort', 'name', true); return false;\">Job Name</a></th>`;
                table += `<th><a href=\"#\" class=\"sort_column\" onclick=\"FilterUtils.applyFilterAndReload('sort', 'description', true); return false;\">Description</a></th>`;
                table += `<th><a href=\"#\" class=\"sort_column\" onclick=\"FilterUtils.applyFilterAndReload('sort', 'status', true); return false;\">Status</a></th>`;
                table += `<th><a href=\"#\" class=\"sort_column\" onclick=\"FilterUtils.applyFilterAndReload('sort', 'command', true); return false;\">Command</a></th>`;
                table += `<th><a href=\"#\" class=\"sort_column\" onclick=\"FilterUtils.applyFilterAndReload('sort', 'next_run', true); return false;\">Next Run</a></th>`;
                table += `<th>Runs</th>`;
                table += '</tr></thead><tbody>';

                // Add table rows
                data.forEach(item => {
                    let next_run = item["next_run"];//.$date.$numberLong;
                    table += '<tr>';
                    table += `<td><input type="checkbox" class="item-checkbox" data-id="${item["_id"]['$oid']}"></td>`;
                    table += `<td>${item["name"]}</td>`;
                    table += `<td>${item["description"]}</td>`;
                    let statusText = "";
                    let statusColor = "";
                    switch (item["status"]) {
                        case 0:
                            statusText = "Unscheduled";
                            statusColor = "orange";
                            break;
                        case 1:
                            statusText = "Running";
                            statusColor = "blue";
                            break;
                        case 2:
                            statusText = "Completed";
                            statusColor = "green";
                            break;
                        case 3:
                            statusText = "Error";
                            statusColor = "red";
                            break;
                        default:
                            statusText = item["status"];
                            statusColor = "";
                    }
                    table += `<td style="color:${statusColor};">${statusText}</td>`;
                    const command = item["command"] || "";
                    const shortCommand = command.length > 10 ? command.substring(0, 10) + "..." : command;
                    const commandId = `command-${item["_id"]['$oid']}`;
                    table += `<td>
                        <span id="${commandId}" style="cursor:pointer;" onclick="
                            const el = document.getElementById('${commandId}');
                            if (el.dataset.expanded === 'true') {
                                el.textContent = '${shortCommand.replace(/'/g, "\\'")}';
                                el.dataset.expanded = 'false';
                            } else {
                                el.textContent = '${command.replace(/'/g, "\\'").replace(/"/g, '&quot;')}';
                                el.dataset.expanded = 'true';
                            }
                        " data-expanded="false">${shortCommand}</span>
                    </td>`;
                    table += `<td class="utc-date" data-timestamp="${next_run}">${next_run}</td>`;
                    table += '<td><button class="btn btn-primary" onclick="#">View Runs</button></td>'
                    table += '</tr>';
                });

                table += '</tbody></table>';

                pagination = "<div class=\"pagination_controls\" id=\"pagination-controls\" style=\"margin-top: 20px;\"></div>";

                container.innerHTML = table + pagination;

                renderPaginationControls(current_page, total_pages);

                DateTimeUtils.convertUtcDateElements();
            }

            // Auto-refresh the table every 10 seconds
            setTimeout(() => renderJobsTable(params), 10000);
        })
        .catch(error => {
            const container = document.getElementById("items");
            if (container) {
                container.innerHTML = `<p>Error loading data: ${error.message}</p>`;
            }
            setTimeout(() => renderJobsTable(params), 10000);
        });
}
