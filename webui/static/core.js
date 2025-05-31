document.addEventListener('DOMContentLoaded', function() {
    // Select all elements with the 'utc-date' class
    const dateCells = document.querySelectorAll('.utc-date');

    dateCells.forEach(cell => {
        const timestamp = parseInt(cell.dataset.timestamp); // Get the timestamp from data-timestamp
        if (!isNaN(timestamp)) {
            const date = new Date(timestamp); // Create a Date object from the milliseconds timestamp

            // Format it as a UTC string
            // Option 1: Standard UTC string
            // const utcString = date.toUTCString();

            // Option 2: ISO 8601 UTC string (often preferred for consistency)
            // const utcString = date.toISOString();

            // Option 3: More readable custom UTC format using Intl.DateTimeFormat
            const options = {
                year: 'numeric',
                month: 'short',
                day: 'numeric',
                hour: '2-digit',
                minute: '2-digit',
                second: '2-digit',
                hour12: false, // Use 24-hour format
                timeZone: 'UTC', // Ensure it's UTC
                timeZoneName: 'short' // e.g., "GMT" or "UTC"
            };
            const utcString = new Intl.DateTimeFormat('en-US', options).format(date);


            cell.textContent = utcString; // Replace the cell's content with the formatted date
        }
    });
});

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
                startDate.getUTCFullYear(),
                startDate.getUTCMonth(),
                startDate.getUTCDate(),
                startDate.getUTCHours(),
                startDate.getUTCMinutes(),
                startDate.getUTCSeconds(),
                startDate.getUTCMilliseconds()
            ); // This is epoch milliseconds in UTC
            url.searchParams.set('range_start', rangeStartMs);
        }
    }
    const rangeEndInput = document.getElementById('range_end');
    if (rangeEndInput && rangeEndInput.value.trim() !== '') {
        const endDate = new Date(rangeEndInput.value.trim());
        if (!isNaN(endDate.getTime())) {
            let rangeEndMs = Date.UTC(
                EndDate.getUTCFullYear(),
                EndDate.getUTCMonth(),
                EndDate.getUTCDate(),
                EndDate.getUTCHours(),
                EndDate.getUTCMinutes(),
                EndDate.getUTCSeconds(),
                EndDate.getUTCMilliseconds()
            ); // This is epoch milliseconds in UTC
            url.searchParams.set('range_end', rangeEndMs);
        }
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
