function renderAgentsTable(params = {}) {
    // Append filter string to the URL if provided
    const url = "/agents_data";
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

            let div = '<div class="agents-list">';

            data.forEach(item => {
                div += '<div class="agent-card';
                if(item["status"] == 1) {
                    div += ' agent-online';
                } else {
                    div += ' agent-offline';       
                }
                div += '">';
                div += item["name"] + '<br>';
                div += `<img width="100px;" src="/agent.png"><br>`;
                div += `<span class="agent-host-info">${item["hostname"]}:${item["port"]}</span><br>`;
                div += '<div class="agent-online-info">';
                div += `Last Ping: <span class="utc-date" data-timestamp="${item["last_ping"]["$date"]["$numberLong"]}">${item["last_ping"]["$date"]["$numberLong"]}</span><br><br>`;
                if (item["status"] == 1) {
                    div += 'Online';
                } else {
                    div += 'Offline';
                }
                div += '</div>'; // Close agent-online-info
                div += '</div>';
            });

            div += '</div>';

            pagination = "<div class=\"pagination_controls\" id=\"pagination-controls\" style=\"margin-top: 20px;\"></div>";

            container.innerHTML = div + pagination;

            renderPaginationControls(current_page, total_pages);

            DateTimeUtils.convertUtcDateElements();

            // Auto-refresh the table every 10 seconds
            setTimeout(() => renderAgentsTable(params), 10000);
        })
        .catch(error => {
            const container = document.getElementById("items");
            if (container) {
                container.innerHTML = `<p>Error loading data: ${error.message}</p>`;
            }
        });
}
