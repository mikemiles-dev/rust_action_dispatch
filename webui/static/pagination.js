function renderPaginationControls(currentPage = 1, totalPages = 1) {
    const container = document.getElementById("pagination-controls");
    if (!container) return;

    let html = '';

    // Show previous button if not on first page
    html += '<div class="pagination_prev">';
    if (currentPage > 1) {
        html += `<a href="#" class="pagination-link btn" data-page="${currentPage - 1}">&laquo; Prev</a>`;
    }
    html += '</div>';

    html += '<div class="pagination_center">';
    // Show up to 5 page numbers, centered around the current page when possible
    const maxPagesToShow = 5;
    let startPage = Math.max(1, currentPage - Math.floor(maxPagesToShow / 2));
    let endPage = startPage + maxPagesToShow - 1;
    if (endPage > totalPages) {
        endPage = totalPages;
        startPage = Math.max(1, endPage - maxPagesToShow + 1);
    }

    if (startPage > 1) {
        html += `<a href="#" class="pagination-link" data-page="1">1</a>`;
        if (startPage > 2) {
            html += `... `;
        }
    }

    for (let i = startPage; i <= endPage; i++) {
        if (i === currentPage) {
            html += `${i} `;
        } else {
            html += `<a href="#" class="pagination-link" data-page="${i}">${i}</a> `;
        }
    }

    if (endPage < totalPages) {
        if (endPage < totalPages - 1) {
            html += `...`;
        }
        html += `<a href="#" class="pagination-link" data-page="${totalPages}">${totalPages}</a> `;
    }
    html += '</div>'; // Close pagination_center

    // Show next button if not on last page
    html += '<div class="pagination_next">';
    if (currentPage < totalPages) {
        html += `<a href="#" class="pagination-link btn" data-page="${currentPage + 1}">Next &raquo;</a>`;
    }
    html += '</div>';

    container.innerHTML = html;

    // Add click handlers
    container.querySelectorAll('.pagination-link').forEach(link => {
        link.addEventListener('click', function (e) {
            e.preventDefault();
            const page = parseInt(this.getAttribute('data-page'), 10);
            const url = new URL(window.location);
            url.searchParams.set('page', page);
            window.location = url.toString();
        });
    });
}