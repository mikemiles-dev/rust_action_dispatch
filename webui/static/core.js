// Global configuration for time format preference
window.prefer12HourFormat = true; // Set to true for 12-hour format, false for 24-hour format

function convertUtcDateElements() {
    // Select all elements with the 'utc-date' class
    const dateCells = document.querySelectorAll('.utc-date');

    dateCells.forEach(cell => {
        const timestamp = cell.dataset.timestamp && !isNaN(cell.dataset.timestamp) ? parseInt(cell.dataset.timestamp) : null; // Safely parse the timestamp
        if (!isNaN(timestamp)) {
            const date = new Date(timestamp); // Create a Date object from the milliseconds timestamp

            // Format it as a UTC string
            // Option 1: Standard UTC string
            // const utcString = date.toUTCString();

            // Option 2: ISO 8601 UTC string (often preferred for consistency)
            // const utcString = date.toISOString();

            // Option 3: More readable custom UTC format using Intl.DateTimeFormat
            /**
             * Options for formatting a date using Intl.DateTimeFormat.
             *
             * @typedef {Object} DateTimeFormatOptions
             * @property {'numeric'} year - Display the year as a numeric value (e.g., "2024").
             * @property {'short'} month - Display the month as a short string (e.g., "Jan").
             * @property {'numeric'} day - Display the day as a numeric value (e.g., "01").
             * @property {'2-digit'} hour - Display the hour as a two-digit value (e.g., "09").
             * @property {'2-digit'} minute - Display the minute as a two-digit value (e.g., "05").
             * @property {'2-digit'} second - Display the second as a two-digit value (e.g., "07").
             * @property {boolean} hour12 - Whether to use 12-hour time format. Uses 24-hour format if false.
             * @property {'UTC'} timeZone - The time zone to use for formatting. Set to 'UTC'.
             * @property {'short'} timeZoneName - Display the time zone name in short form (e.g., "UTC").
             */
            const options = {
                year: 'numeric',
                month: 'short',
                day: 'numeric',
                hour: '2-digit',
                minute: '2-digit',
                second: '2-digit',
                hour12: window.prefer12HourFormat || false, // Use 12-hour format if configured, default to 24-hour
                timeZone: 'UTC', // Ensure it's UTC
                timeZoneName: 'short' // e.g., "GMT" or "UTC"
            };
            const utcString = new Intl.DateTimeFormat('en-US', options).format(date);


            cell.textContent = utcString; // Replace the cell's content with the formatted date
        }
    });
};

function setInputTime(element_id, utcEpochMs) {
    // Exit early if the URL parameter with the same name as element_id is not set
    const url = new URL(window.location.href);
    if (!url.searchParams.has(element_id)) {
        return;
    }
    // 1. Create a Date object from the UTC epoch milliseconds.
    //    The Date constructor treats this as UTC milliseconds.
    const date = new Date(utcEpochMs);

    // 2. Format the Date object into the YYYY-MM-DDTHH:MM string for datetime-local.
    //    Crucially, datetime-local expects the *local* date and time, so we use
    //    the local methods of the Date object to get the components.
    //    The browser then displays this local time to the user.
    const year = date.getUTCFullYear();
    const month = String(date.getUTCMonth() + 1).padStart(2, '0'); // Months are 0-indexed
    const day = String(date.getUTCDate()).padStart(2, '0');
    const hours = String(date.getUTCHours()).padStart(2, '0');
    const minutes = String(date.getUTCMinutes()).padStart(2, '0');

    const formattedDateTime = `${year}-${month}-${day}T${hours}:${minutes}`;

    // 3. Set the value of the input element
    document.getElementById(element_id).value = formattedDateTime;
}

function incrementPageAndReload() {
    const url = new URL(window.location.href);
    const currentPage = parseInt(url.searchParams.get('page')) || 1;
    url.searchParams.set('page', currentPage + 1);
    window.location.href = url.toString();
}

function decrementPageAndReload() {
    const url = new URL(window.location.href);
    const currentPage = parseInt(url.searchParams.get('page')) || 1;
    if (currentPage > 1) {
        url.searchParams.set('page', currentPage - 1);
        window.location.href = url.toString();
    }
}

function goToPage(pageNumber) {
    const url = new URL(window.location.href);
    url.searchParams.set('page', pageNumber);
    window.location = url.toString();
}

/**
 * Applies a filter to the current page by updating the URL query parameters and reloads the page.
 * Optionally handles date range filters, resets pagination, and toggles sort order.
 *
 * @param {string} filterName - The name of the filter to apply (used as the query parameter key).
 * @param {string} filterValue - The value of the filter to apply.
 * @param {boolean} [change_order=false] - If true, toggles the 'order' query parameter between 'asc' and 'desc'.
 * @param {boolean} [resetPage=false] - If true, resets the 'page' query parameter to 1.
 *
 * @remarks
 * - If input fields with IDs 'range_start' or 'range_end' are present and have values, their values are converted to epoch milliseconds and set as query parameters.
 * - If the date range inputs are empty, their corresponding query parameters are removed.
 * - The function updates the browser's location, causing a page reload with the new query parameters.
 */
function applyFilterAndReload(filterName, filterValue, change_order = false, resetPage = false) {
    const url = new URL(window.location.href);
    url.searchParams.set(filterName, filterValue);

    // Check for range_start and range_end input fields and set query params if not empty
    const rangeStartInput = document.getElementById('range_start');
    // Convert date string to epoch milliseconds if value is not empty
    if (rangeStartInput && rangeStartInput.value.trim() !== '') {
        const startDate = new Date(rangeStartInput.value.trim());
        if (!isNaN(startDate.getTime())) {
            let rangeStartMs = Date.UTC(
                startDate.getFullYear(),
                startDate.getMonth(),
                startDate.getDate(),
                startDate.getHours(),
                startDate.getMinutes(),
                startDate.getSeconds(),
                startDate.getMilliseconds()
            ); 
            url.searchParams.set('range_start', rangeStartMs);
        }
    } else {
        // If range_end is empty, remove it from the URL
        url.searchParams.delete('range_start');
    }
    const rangeEndInput = document.getElementById('range_end');
    if (rangeEndInput && rangeEndInput.value.trim() !== '') {
        const endDate = new Date(rangeEndInput.value.trim());
        if (!isNaN(endDate.getTime())) {
            let rangeEndMs = Date.UTC(
                endDate.getFullYear(),
                endDate.getMonth(),
                endDate.getDate(),
                endDate.getHours(),
                endDate.getMinutes(),
                endDate.getSeconds(),
                endDate.getMilliseconds()
            ); 
            url.searchParams.set('range_end', rangeEndMs);
        }
    } else {
        // If range_end is empty, remove it from the URL
        url.searchParams.delete('range_end');
    }
    if (resetPage) {
        url.searchParams.set('page', 1); // Reset to page 1 if specified
    }
    // Toggle 'order' query parameter between 'asc' and 'desc'
    if (change_order) {
        const currentOrder = url.searchParams.get('order');
        if (currentOrder === 'asc') {
            url.searchParams.set('order', 'desc');
        } else if (currentOrder === 'desc') {
            url.searchParams.set('order', 'asc');
        } else {
            url.searchParams.set('order', 'asc'); // Default to ascending if not set
        }
    }
    window.location.href = url.toString();
}

/**
 * Makes an AJAX GET request to the specified URL and expects JSON data in response.
 * @param {string} url - The URL to send the GET request to.
 * @returns {Promise<any>} - A promise that resolves with the parsed JSON data.
 */
function getJsonData(url, params = {}) {
    // Remove keys with empty string values from params
    const filteredParams = Object.fromEntries(
        Object.entries(params).filter(([_, v]) => v !== '')
    );

    const currentUrlParams = Object.fromEntries(new URL(window.location.href).searchParams.entries());

    if (!('range_start' in currentUrlParams)) {
        delete filteredParams['range_start'];
    }
    if (!('range_end' in currentUrlParams)) {
        delete filteredParams['range_end'];
    }

    const queryString = Object.keys(filteredParams).length
        ? '?' + new URLSearchParams(filteredParams).toString()
        : '';
    const fullUrl = url + queryString;

    return fetch(fullUrl, {
        method: 'GET',
        headers: {
            'Accept': 'application/json'
        }
    })
    .then(response => {
        if (!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`);
        }
        return response.json();
    });
}

function renderRunsTable(params = {}) {
    // Append filter string to the URL if provided
    const url = "/runs_data";
    getJsonData(url, params)
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
            table += `<th>Job Name</th>`;
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

            convertUtcDateElements();
        })
        .catch(error => {
            const container = document.getElementById("items");
            if (container) {
                container.innerHTML = `<p>Error loading data: ${error.message}</p>`;
            }
        });
}